use crate::inject::{InjectRequest, Injector, NEXT_PROMPT};
use crate::session::Target;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Crack,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub id: String,
    pub path: String,
}

pub trait Audio {
    fn play_whip(&mut self);
    fn play_next(&mut self);
}

#[derive(Default)]
pub struct RecordingAudio {
    pub whip: usize,
    pub next: usize,
}

impl Audio for RecordingAudio {
    fn play_whip(&mut self) {
        self.whip += 1;
    }
    fn play_next(&mut self) {
        self.next += 1;
    }
}

pub struct WhipThenNext<I: Injector, A: Audio> {
    pub idle: Vec<Frame>,
    pub crack: Vec<Frame>,
    pub injector: I,
    pub audio: A,
    pub resolve: Box<dyn Fn() -> Option<Target>>,
    pub phase: Phase,
    pub index: usize,
    pub payload: String,
    injected: bool,
}

impl<I: Injector, A: Audio> WhipThenNext<I, A> {
    pub fn new(
        idle: Vec<Frame>,
        crack: Vec<Frame>,
        injector: I,
        audio: A,
        resolve: Box<dyn Fn() -> Option<Target>>,
    ) -> Self {
        assert!(!idle.is_empty(), "need idle frames");
        assert!(crack.len() >= 2, "crack sequence needs more than one frame");
        Self {
            idle,
            crack,
            injector,
            audio,
            resolve,
            phase: Phase::Idle,
            index: 0,
            payload: NEXT_PROMPT.to_string(),
            injected: false,
        }
    }

    pub fn frame_id(&self) -> &str {
        match self.phase {
            Phase::Idle => &self.idle[self.index % self.idle.len()].id,
            Phase::Crack => &self.crack[self.index].id,
        }
    }

    pub fn frame_path(&self) -> &str {
        match self.phase {
            Phase::Idle => &self.idle[self.index % self.idle.len()].path,
            Phase::Crack => &self.crack[self.index].path,
        }
    }

    /// Shipped click handler.
    pub fn handle_click(&mut self) -> bool {
        if self.phase != Phase::Idle {
            return false;
        }
        self.phase = Phase::Crack;
        self.index = 0;
        self.injected = false;
        self.audio.play_whip();
        true
    }

    pub fn tick(&mut self) {
        if self.phase == Phase::Idle {
            self.index = (self.index + 1) % self.idle.len();
            return;
        }
        if self.index + 1 < self.crack.len() {
            self.index += 1;
            return;
        }
        self.finish_crack();
    }

    fn finish_crack(&mut self) {
        if let Some(target) = (self.resolve)() {
            self.injector.submit(InjectRequest {
                target,
                payload: crate::inject::sanitize_phrase(&self.payload),
                submit: true,
            });
        }
        self.injected = true;
        self.phase = Phase::Idle;
        self.index = 0;
    }
}

pub fn run_shipped_click<I: Injector, A: Audio>(machine: &mut WhipThenNext<I, A>) -> Vec<String> {
    machine.handle_click();
    let mut seen = vec![machine.frame_id().to_string()];
    while machine.phase == Phase::Crack {
        machine.tick();
        seen.push(machine.frame_id().to_string());
    }
    seen
}
