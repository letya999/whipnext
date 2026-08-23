use std::path::{Path, PathBuf};

use crate::session::Target;

pub const NEXT_PROMPT: &str = "next";

type SpawnHerdr<'a> = &'a mut dyn FnMut(&str, &[String]) -> Result<(), String>;
type PidInject<'a> = &'a mut dyn FnMut(u32, &str) -> Result<(), String>;
type HerdrRun = Box<dyn FnMut(&str, &[String])>;

/// Character phrases may be any non-empty text except a slash command.
pub fn sanitize_phrase(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() || t.starts_with('/') {
        NEXT_PROMPT.to_string()
    } else {
        t.to_string()
    }
}

/// Virtual-key sequence for `next` (N E X T) — more reliable in Windows TUIs than UNICODE.
pub const NEXT_VKS: [u16; 4] = [0x4E, 0x45, 0x58, 0x54];

/// US scan codes matching NEXT_VKS (MapVirtualKey MAPVK_VK_TO_VSC).
pub const NEXT_SCANS: [u16; 4] = [0x31, 0x12, 0x2D, 0x14];

/// Generic and left/right Shift, Ctrl, Alt, plus Windows keys.
pub const MODIFIER_VKS: [u16; 11] = [
    0x10, 0x11, 0x12, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5B, 0x5C,
];

/// Wait after attaching / before the first letter so a TUI does not drop it.
pub const INJECT_ATTACH_MS: u32 = 80;
/// Always wait before the first letter, even if the TUI already has focus.
/// Codex drops the first keys if typing starts immediately after the overlay click.
pub const INJECT_SETTLE_MS: u32 = 150;
/// Let the terminal consume synthetic modifier releases before paste.
pub const INJECT_MODIFIER_MS: u32 = 50;
/// Gap between a key-down and its key-up.
pub const INJECT_DOWN_UP_MS: u32 = 12;
/// Gap between typed keys.
pub const INJECT_KEY_MS: u32 = 35;
/// Wait after the phrase, before Enter.
pub const INJECT_ENTER_MS: u32 = 80;
/// Wait after Ctrl+V so the TUI reads the clipboard before we restore it.
pub const INJECT_PASTE_MS: u32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedKey {
    Vk(u16),
    Unicode(char),
}

/// ASCII letters/digits/space as VKs (TUIs), everything else as UNICODE.
pub fn is_vk_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == ' '
}

pub fn typed_keys(text: &str) -> Vec<TypedKey> {
    text.chars()
        .map(|ch| {
            if is_vk_char(ch) {
                TypedKey::Vk(ch.to_ascii_uppercase() as u16)
            } else {
                TypedKey::Unicode(ch)
            }
        })
        .collect()
}

/// How to deliver the phrase. Windows Terminal drops KEYEVENTF_UNICODE, so
/// Cyrillic typed that way never arrives (looks like a truncated phrase).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeMethod {
    /// `next` / `keep going` — US-layout virtual keys.
    Keys,
    /// Character exists on the active keyboard layout (Russian on a RU layout).
    Layout,
    /// Character is not on the layout — clipboard Ctrl+V.
    Paste,
}

pub fn type_method(_text: &str, _layout_ok: bool) -> TypeMethod {
    TypeMethod::Paste
}

/// Packed `VkKeyScanEx` result: low byte VK, high byte shift/ctrl/alt.
/// `None` if missing or if Ctrl/Alt would be required (those are shortcuts).
pub fn layout_key(scan: i16) -> Option<(u16, bool)> {
    if scan < 0 {
        return None;
    }
    let bits = scan as u16;
    let vk = bits & 0xFF;
    let mods = (bits >> 8) & 0xFF;
    if mods & 0xFE != 0 {
        return None;
    }
    Some((vk, mods & 1 != 0))
}

pub fn layout_ok_for(text: &str, mut lookup: impl FnMut(char) -> Option<(u16, bool)>) -> bool {
    text.chars().all(|ch| lookup(ch).is_some())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevSink {
    pub log_path: PathBuf,
    pub last_path: PathBuf,
}

pub fn dev_sink(home: &Path) -> DevSink {
    DevSink {
        log_path: crate::settings::dev_log_path(home),
        last_path: crate::settings::dev_last_path(home),
    }
}

pub fn write_dev(sink: &DevSink, v: &serde_json::Value) {
    use std::io::Write;
    let line = v.to_string();
    println!("{line}");
    if let Some(dir) = sink.log_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&sink.log_path)
    {
        let _ = writeln!(f, "{line}");
    }
    let pretty = serde_json::to_string_pretty(v).unwrap_or(line);
    let _ = std::fs::write(&sink.last_path, pretty + "\n");
}

/// Restore a minimized window before typing. Never restore a visible
/// fullscreen/maximized window — `SW_RESTORE` drops it to windowed.
pub fn focus_needs_restore(minimized: bool) -> bool {
    minimized
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InjectRequest {
    pub target: Target,
    pub payload: String,
    pub submit: bool,
}

pub trait Injector {
    fn submit(&mut self, req: InjectRequest);
}

#[derive(Default)]
pub struct RecordingInjector {
    pub calls: Vec<InjectRequest>,
}

impl Injector for RecordingInjector {
    fn submit(&mut self, req: InjectRequest) {
        assert!(
            !req.payload.starts_with('/'),
            "payload must not be a slash command"
        );
        assert!(!req.payload.trim().is_empty(), "payload must not be empty");
        self.calls.push(req);
    }
}

/// How the production adapter would invoke Herdr. Tests assert this argv.
pub fn herdr_prompt_command(herdr_bin: &str, pane_or_name: &str) -> (String, Vec<String>) {
    herdr_prompt_command_with(herdr_bin, pane_or_name, NEXT_PROMPT)
}

pub fn herdr_prompt_command_with(
    herdr_bin: &str,
    pane_or_name: &str,
    phrase: &str,
) -> (String, Vec<String>) {
    (
        herdr_bin.to_string(),
        crate::herdr::prompt_args(pane_or_name, phrase),
    )
}

/// What `live_inject` will try, in order: addressed Herdr pane first, then OS fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveAttempt {
    Pid(u32),
    Herdr { pane: String },
}

pub fn live_attempts(target: &Target) -> Vec<LiveAttempt> {
    let mut out = Vec::new();
    if let Some(pane) = target.pane_id.as_deref().or(target.herdr_name.as_deref()) {
        out.push(LiveAttempt::Herdr {
            pane: pane.to_string(),
        });
    }
    if let Some(pid) = target.pid {
        out.push(LiveAttempt::Pid(pid));
    }
    out
}

/// Shipped inject policy without OS FFI. First success wins.
pub fn execute_live(
    target: &Target,
    inject_pid: PidInject<'_>,
    spawn_herdr: SpawnHerdr<'_>,
) -> Result<(), String> {
    execute_live_payload(target, NEXT_PROMPT, inject_pid, spawn_herdr)
}

pub fn execute_live_payload(
    target: &Target,
    payload: &str,
    inject_pid: PidInject<'_>,
    spawn_herdr: SpawnHerdr<'_>,
) -> Result<(), String> {
    let payload = sanitize_phrase(payload);
    let attempts = live_attempts(target);
    if attempts.is_empty() {
        return Err("no pane_id or pid on target".into());
    }
    let mut last = String::from("no pane_id or pid on target");
    for attempt in attempts {
        match attempt {
            LiveAttempt::Pid(pid) => match inject_pid(pid, &payload) {
                Ok(()) => return Ok(()),
                Err(e) => last = e,
            },
            LiveAttempt::Herdr { pane } => {
                let (bin, args) = herdr_prompt_command_with("herdr", &pane, &payload);
                match spawn_herdr(&bin, &args) {
                    Ok(()) => return Ok(()),
                    Err(e) => last = format!("herdr: {e}"),
                }
            }
        }
    }
    Err(last)
}

pub struct CommandInjector {
    pub herdr_bin: String,
    pub run: HerdrRun,
}

impl Injector for CommandInjector {
    fn submit(&mut self, req: InjectRequest) {
        assert!(!req.payload.starts_with('/'));
        let pane = req
            .target
            .pane_id
            .as_deref()
            .or(req.target.herdr_name.as_deref());
        let Some(pane) = pane else {
            return;
        };
        let (bin, args) = herdr_prompt_command_with(&self.herdr_bin, pane, &req.payload);
        (self.run)(&bin, &args);
    }
}
