use std::path::{Path, PathBuf};

/// Coding-agent kinds this binary uniquely understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Grok,
    Claude,
    Codex,
    Antigravity,
    Pi,
    OpenCode,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Grok,
        Kind::Claude,
        Kind::Codex,
        Kind::Antigravity,
        Kind::Pi,
        Kind::OpenCode,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Grok => "grok",
            Kind::Claude => "claude",
            Kind::Codex => "codex",
            Kind::Antigravity => "antigravity",
            Kind::Pi => "pi",
            Kind::OpenCode => "opencode",
        }
    }

    /// Herdr `--kind` / `agent` field.
    pub fn herdr_kind(self) -> &'static str {
        match self {
            Kind::Grok => "grok",
            Kind::Claude => "claude",
            Kind::Codex => "codex",
            Kind::Antigravity => "agy",
            Kind::Pi => "pi",
            Kind::OpenCode => "opencode",
        }
    }

    pub fn from_herdr(s: &str) -> Option<Kind> {
        match s.to_ascii_lowercase().as_str() {
            "grok" => Some(Kind::Grok),
            "claude" => Some(Kind::Claude),
            "codex" => Some(Kind::Codex),
            "agy" | "antigravity" => Some(Kind::Antigravity),
            "pi" => Some(Kind::Pi),
            "opencode" => Some(Kind::OpenCode),
            _ => None,
        }
    }

    pub fn from_binary_name(name: &str) -> Option<Kind> {
        let n = name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(name)
            .trim_end_matches(".exe")
            .trim_end_matches(".EXE")
            .trim_end_matches(".cmd")
            .trim_end_matches(".ps1")
            .to_ascii_lowercase();
        match n.as_str() {
            "grok" => Some(Kind::Grok),
            "claude" => Some(Kind::Claude),
            "codex" => Some(Kind::Codex),
            "agy" | "antigravity" => Some(Kind::Antigravity),
            "pi" => Some(Kind::Pi),
            "opencode" => Some(Kind::OpenCode),
            _ => None,
        }
    }

    pub fn binaries(self) -> &'static [&'static str] {
        match self {
            Kind::Grok => &["grok", "grok.exe"],
            Kind::Claude => &["claude", "claude.exe"],
            Kind::Codex => &["codex", "codex.exe", "codex.cmd", "codex.ps1"],
            Kind::Antigravity => &["agy", "agy.exe", "antigravity", "antigravity.exe"],
            Kind::Pi => &["pi", "pi.exe"],
            Kind::OpenCode => &["opencode", "opencode.exe"],
        }
    }
}

pub fn extra_bin_dirs(home: &Path, local_app: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = vec![
        home.join(".grok").join("bin"),
        home.join(".local").join("bin"),
        home.join(".cargo").join("bin"),
    ];
    if let Some(la) = local_app {
        dirs.push(la.join("agy").join("bin"));
        dirs.push(la.join("pi"));
        dirs.push(la.join("Programs").join("opencode"));
    }
    dirs
}
