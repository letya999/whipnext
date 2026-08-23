use whipnext::inject::{InjectRequest, RecordingInjector, NEXT_PROMPT};
use whipnext::kind::Kind;
use whipnext::session::Target;
use whipnext::whip::{run_shipped_click, Audio, Frame, Phase, RecordingAudio, WhipThenNext};

fn frames() -> (Vec<Frame>, Vec<Frame>) {
    let idle = vec![
        Frame {
            id: "idle_00.png".into(),
            path: "idle_00.png".into(),
        },
        Frame {
            id: "idle_01.png".into(),
            path: "idle_01.png".into(),
        },
    ];
    let crack = vec![
        Frame {
            id: "punch_00.png".into(),
            path: "punch_00.png".into(),
        },
        Frame {
            id: "punch_01.png".into(),
            path: "punch_01.png".into(),
        },
        Frame {
            id: "punch_02.png".into(),
            path: "punch_02.png".into(),
        },
        Frame {
            id: "punch_03.png".into(),
            path: "punch_03.png".into(),
        },
    ];
    (idle, crack)
}

#[test]
fn click_cracks_then_injects_next_once_with_sounds() {
    let (idle, crack) = frames();
    let target = Target {
        kind: Kind::Grok,
        session_id: Some("front-grok".into()),
        pid: Some(222),
        pane_id: Some("wG:p4".into()),
        herdr_name: Some("worker-1".into()),
    };
    let t2 = target.clone();
    let mut machine = WhipThenNext::new(
        idle,
        crack,
        RecordingInjector::default(),
        RecordingAudio::default(),
        Box::new(move || Some(t2.clone())),
    );
    let seen = run_shipped_click(&mut machine);
    let punch: Vec<_> = seen
        .iter()
        .filter(|id| id.starts_with("punch_"))
        .cloned()
        .collect();
    assert!(punch.len() > 1);
    assert_eq!(
        punch,
        vec![
            "punch_00.png",
            "punch_01.png",
            "punch_02.png",
            "punch_03.png"
        ]
    );
    assert_eq!(machine.phase, Phase::Idle);
    assert_eq!(machine.audio.whip, 1);
    assert_eq!(machine.audio.next, 0);
    assert_eq!(machine.injector.calls.len(), 1);
    let call = &machine.injector.calls[0];
    assert_eq!(
        call,
        &InjectRequest {
            target,
            payload: NEXT_PROMPT.into(),
            submit: true,
        }
    );
    assert_eq!(call.payload, "next");
    assert_ne!(call.payload, "/next");
    assert_eq!(call.target.pane_id.as_deref(), Some("wG:p4"));
}

#[test]
fn shipped_click_injects_character_phrase_not_slash() {
    let (idle, crack) = frames();
    let mut machine = WhipThenNext::new(
        idle,
        crack,
        RecordingInjector::default(),
        RecordingAudio::default(),
        Box::new(|| {
            Some(Target {
                kind: Kind::Grok,
                session_id: None,
                pid: Some(1),
                pane_id: Some("wG:p4".into()),
                herdr_name: None,
            })
        }),
    );
    machine.payload = "keep going".into();
    run_shipped_click(&mut machine);
    assert_eq!(machine.injector.calls[0].payload, "keep going");
    assert!(!machine.injector.calls[0].payload.starts_with('/'));

    machine.payload = "/next".into();
    assert!(machine.handle_click());
    while machine.phase == Phase::Crack {
        machine.tick();
    }
    assert_eq!(machine.injector.calls[1].payload, NEXT_PROMPT);
}

#[test]
fn click_while_cracking_does_not_reinject() {
    let (idle, crack) = frames();
    let mut machine = WhipThenNext::new(
        idle,
        crack,
        RecordingInjector::default(),
        RecordingAudio::default(),
        Box::new(|| {
            Some(Target {
                kind: Kind::Grok,
                session_id: None,
                pid: Some(1),
                pane_id: Some("wG:p4".into()),
                herdr_name: None,
            })
        }),
    );
    assert!(machine.handle_click());
    assert!(!machine.handle_click());
    while machine.phase == Phase::Crack {
        machine.tick();
    }
    assert_eq!(machine.injector.calls.len(), 1);
    assert_eq!(machine.audio.whip, 1);
}

#[test]
fn click_without_session_does_not_panic() {
    let (idle, crack) = frames();
    let mut machine = WhipThenNext::new(
        idle,
        crack,
        RecordingInjector::default(),
        RecordingAudio::default(),
        Box::new(|| None),
    );
    assert!(machine.handle_click());
    while machine.phase == Phase::Crack {
        machine.tick();
    }
    assert!(machine.injector.calls.is_empty());
    assert_eq!(machine.audio.whip, 1);
    assert_eq!(machine.audio.next, 0);
}

#[test]
fn idle_tick_advances_frame_path() {
    let (idle, crack) = frames();
    let mut machine = WhipThenNext::new(
        idle,
        crack,
        RecordingInjector::default(),
        RecordingAudio::default(),
        Box::new(|| None),
    );
    let a = machine.frame_path().to_string();
    machine.tick();
    let b = machine.frame_path().to_string();
    assert_ne!(a, b);
    assert!(machine.frame_id().starts_with("idle_"));
    assert!(machine.handle_click());
    assert!(machine.frame_path().contains("punch") || machine.frame_path().contains("action"));
    machine.audio.play_next();
    assert_eq!(machine.audio.next, 1);
}
