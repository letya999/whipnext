//! Should the girl be visible for this foreground window?

use crate::settings::Settings;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proc {
    pub pid: u32,
    pub name: String,
    pub parent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Foreground {
    pub pid: u32,
    pub exe: String,
    pub title: String,
    pub procs: Vec<Proc>,
}

fn norm_exe(name: &str) -> String {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_ascii_lowercase()
}

/// Foreground pid plus its ancestors and descendants (host terminal + agent child).
pub fn related_pids(fg: &Foreground) -> Vec<u32> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut push = |pid: u32| {
        if seen.insert(pid) {
            out.push(pid);
        }
    };
    push(fg.pid);
    for p in descendants(fg.pid, &fg.procs) {
        push(p.pid);
    }
    let mut pid = fg.pid;
    let mut hops = std::collections::HashSet::new();
    while hops.insert(pid) {
        let Some(p) = fg.procs.iter().find(|x| x.pid == pid) else {
            break;
        };
        if p.parent == 0 || p.parent == pid {
            break;
        }
        pid = p.parent;
        push(pid);
    }
    out
}

fn descendants(root: u32, procs: &[Proc]) -> Vec<&Proc> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    let mut seen = std::collections::HashSet::new();
    while let Some(pid) = stack.pop() {
        if !seen.insert(pid) {
            continue;
        }
        for p in procs {
            if p.parent == pid {
                out.push(p);
                stack.push(p.pid);
            }
        }
    }
    out
}

/// Terminal / IDE hosts: the agent often lives in a child process.
pub fn is_host(exe: &str) -> bool {
    matches!(
        norm_exe(exe).as_str(),
        "windowsterminal"
            | "wt"
            | "conhost"
            | "openconsole"
            | "powershell"
            | "pwsh"
            | "cmd"
            | "wezterm"
            | "alacritty"
            | "kitty"
            | "ghostty"
            | "windows.terminal"
            | "code"
            | "code - insiders"
            | "cursor"
            | "windsurf"
            | "zed"
            | "antigravity"
            | "herdr"
    )
}

pub fn is_terminal_host(exe: &str) -> bool {
    matches!(
        norm_exe(exe).as_str(),
        "windowsterminal" | "wt" | "conhost" | "openconsole" | "windows.terminal"
    )
}

pub fn is_self(exe: &str) -> bool {
    matches!(norm_exe(exe).as_str(), "whipnext")
}

pub fn is_herdr_exe(exe: &str) -> bool {
    matches!(norm_exe(exe).as_str(), "herdr")
}

/// Built-in AI surfaces. Games and browsers are never here.
pub fn surface_for_exe(exe: &str) -> Option<&'static str> {
    match norm_exe(exe).as_str() {
        "grok" => Some("grok"),
        "claude" => Some("claude"),
        "codex" => Some("codex"),
        "agy" | "antigravity" => Some("antigravity"),
        "pi" => Some("pi"),
        "opencode" => Some("opencode"),
        "cursor" => Some("cursor"),
        "windsurf" => Some("windsurf"),
        "zed" => Some("zed"),
        _ => None,
    }
}

pub fn surface_from_title(title: &str) -> Option<&'static str> {
    let t = title.to_ascii_lowercase();
    if t.contains("civilization") || t.contains("civ6") || t.contains("civ 6") {
        return None;
    }
    const NEEDLES: &[(&str, &str)] = &[
        ("grok", "grok"),
        ("claude", "claude"),
        ("codex", "codex"),
        ("opencode", "opencode"),
        ("open code", "opencode"),
        ("open-code", "opencode"),
        ("antigravity", "antigravity"),
        ("agy ", "antigravity"),
        ("cursor", "cursor"),
        ("windsurf", "windsurf"),
        (" pi ", "pi"),
    ];
    for (n, id) in NEEDLES {
        if t.contains(n) {
            return Some(id);
        }
    }
    None
}

/// Which pack surface the overlay should follow this frame. `None` hides it.
pub fn overlay_surface(
    always_show: bool,
    fg: Option<&Foreground>,
    last_surface: Option<&str>,
    settings: &Settings,
) -> Option<String> {
    if always_show {
        return Some("always".into());
    }
    let fg = fg?;
    if is_self(&fg.exe) {
        return last_surface
            .filter(|id| *id == "always" || settings.app_enabled(id) || !settings.only_when_matched)
            .map(str::to_string);
    }
    match_foreground(fg, settings)
}

/// None = hide the girl (Civ 6, browser, disabled app, …).
pub fn match_foreground(fg: &Foreground, settings: &Settings) -> Option<String> {
    if !settings.only_when_matched {
        return Some("always".into());
    }
    let direct = surface_for_exe(&fg.exe);
    if let Some(id) = direct {
        if settings.app_enabled(id) {
            return Some(id.to_string());
        }
        return None;
    }
    if is_host(&fg.exe) {
        let mut child_surface = None;
        for child in descendants(fg.pid, &fg.procs) {
            if let Some(id) = surface_for_exe(&child.name) {
                if settings.app_enabled(id) {
                    if child_surface.is_some_and(|current| current != id) {
                        // A host can own several harnesses (for example, Herdr
                        // panes). Process order does not identify the focused one.
                        return None;
                    }
                    child_surface = Some(id);
                }
            }
        }
        if let Some(id) = child_surface {
            return Some(id.to_string());
        }
        if !is_terminal_host(&fg.exe) {
            if let Some(id) = surface_from_title(&fg.title) {
                if settings.app_enabled(id) {
                    return Some(id.to_string());
                }
            }
        }
    }
    None
}
