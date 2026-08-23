//! User settings: which AI surfaces get the girl, which model/sounds.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::detect::AgentStatus;
use crate::layout::{self, DEFAULT_SCALE};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppPref {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Override pack model id for this app only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound_whip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound_next: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for AppPref {
    fn default() -> Self {
        Self {
            enabled: true,
            model: None,
            sound_whip: None,
            sound_next: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayShowMode {
    /// Show whenever a matched surface is focused.
    #[default]
    Always,
    /// Show only while `overlay_key` is held.
    WhileHeld,
    /// Hide the cursor animation while `overlay_key` is held.
    HideWhileHeld,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    /// Pack model folder name under assets/pack/models/
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_whip")]
    pub sound_whip: String,
    #[serde(default = "default_next")]
    pub sound_next: String,
    /// If true, hide the girl unless the foreground app is an enabled AI surface.
    #[serde(default = "default_true")]
    pub only_when_matched: bool,
    /// Temporary hide overlay without changing per-app toggles.
    #[serde(default)]
    pub hide_now: bool,
    /// Overlay pixel size as percent of the base overlay size.
    #[serde(default = "default_scale")]
    pub overlay_scale: u32,
    /// Settings control size as percent (50–200).
    #[serde(default = "default_scale")]
    pub control_size: u32,
    #[serde(default = "default_overlay_mode")]
    pub overlay_mode: OverlayShowMode,
    /// Hold-key name: ctrl, alt, shift, win.
    #[serde(default = "default_overlay_key")]
    pub overlay_key: String,
    /// Per-character inject phrase overrides (keyed by model id).
    #[serde(default)]
    pub phrases: BTreeMap<String, String>,
    /// Per-character hit sound stem (keyed by model id).
    #[serde(default)]
    pub model_sounds: BTreeMap<String, String>,
    /// Settings UI language: `ru` or `en`.
    #[serde(default = "default_locale")]
    pub locale: String,
    /// When true, inject writes `dev.log` + `dev-last-inject.json` under `~/.whipnext/`.
    #[serde(default)]
    pub dev_mode: bool,
    #[serde(default)]
    pub apps: BTreeMap<String, AppPref>,
}

fn default_model() -> String {
    "default".into()
}
fn default_whip() -> String {
    "whip".into()
}
fn default_next() -> String {
    "next".into()
}
fn default_scale() -> u32 {
    DEFAULT_SCALE
}
fn default_overlay_key() -> String {
    "alt".into()
}
fn default_overlay_mode() -> OverlayShowMode {
    OverlayShowMode::Always
}
fn default_locale() -> String {
    "ru".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: default_model(),
            sound_whip: default_whip(),
            sound_next: default_next(),
            only_when_matched: true,
            hide_now: false,
            overlay_scale: default_scale(),
            control_size: default_scale(),
            overlay_mode: OverlayShowMode::Always,
            overlay_key: default_overlay_key(),
            phrases: BTreeMap::new(),
            model_sounds: BTreeMap::new(),
            locale: default_locale(),
            dev_mode: false,
            apps: BTreeMap::new(),
        }
    }
}

impl Settings {
    pub fn app_enabled(&self, id: &str) -> bool {
        self.apps.get(id).map(|a| a.enabled).unwrap_or(false)
    }

    pub fn model_for(&self, id: Option<&str>) -> &str {
        if let Some(id) = id {
            if let Some(m) = self.apps.get(id).and_then(|a| a.model.as_deref()) {
                if !m.is_empty() {
                    return m;
                }
            }
        }
        &self.model
    }

    pub fn sound_whip_for(&self, id: Option<&str>) -> &str {
        self.app_sound_whip_override(id).unwrap_or(&self.sound_whip)
    }

    pub fn sound_next_for(&self, id: Option<&str>) -> &str {
        self.app_sound_next_override(id).unwrap_or(&self.sound_next)
    }

    /// Explicit per-harness whip override, if the user set one.
    pub fn app_sound_whip_override(&self, id: Option<&str>) -> Option<&str> {
        id.and_then(|id| self.apps.get(id))
            .and_then(|a| a.sound_whip.as_deref())
            .filter(|s| !s.is_empty())
    }

    pub fn app_sound_next_override(&self, id: Option<&str>) -> Option<&str> {
        id.and_then(|id| self.apps.get(id))
            .and_then(|a| a.sound_next.as_deref())
            .filter(|s| !s.is_empty())
    }

    /// Hit sound: per-harness override, else per-model assignment, else catalog, else global.
    pub fn resolved_sound_whip(
        &self,
        surface: Option<&str>,
        character_sound: Option<&str>,
    ) -> String {
        if let Some(s) = self.app_sound_whip_override(surface) {
            return s.to_string();
        }
        let model = self.model_for(surface);
        if let Some(s) = self.model_sounds.get(model).filter(|s| !s.is_empty()) {
            return s.clone();
        }
        if let Some(s) = character_sound.filter(|s| !s.is_empty()) {
            return s.to_string();
        }
        self.sound_whip.clone()
    }

    pub fn resolved_sound_next(
        &self,
        surface: Option<&str>,
        character_sound: Option<&str>,
    ) -> String {
        if let Some(s) = self.app_sound_next_override(surface) {
            return s.to_string();
        }
        if let Some(s) = character_sound.filter(|s| !s.is_empty()) {
            return s.to_string();
        }
        self.sound_next.clone()
    }

    /// Phrase for a character id: settings override, else default `next`.
    pub fn phrase_for(&self, model_id: &str) -> String {
        if let Some(p) = self.phrases.get(model_id) {
            return crate::inject::sanitize_phrase(p);
        }
        crate::inject::NEXT_PROMPT.to_string()
    }

    pub fn phrase_for_surface(&self, surface: Option<&str>) -> String {
        let model = self.model_for(surface);
        self.phrase_for(model)
    }
}

pub fn home_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("WHIPNEXT_HOME") {
        return PathBuf::from(d);
    }
    crate::os_probe::os_home(std::env::var_os("USERPROFILE"), std::env::var_os("HOME"))
}

pub fn settings_dir(home: &Path) -> PathBuf {
    home.join(".whipnext")
}

pub fn settings_path(home: &Path) -> PathBuf {
    settings_dir(home).join("settings.json")
}

pub fn dev_log_path(home: &Path) -> PathBuf {
    settings_dir(home).join("dev.log")
}

pub fn dev_last_path(home: &Path) -> PathBuf {
    settings_dir(home).join("dev-last-inject.json")
}

pub fn load(path: &Path) -> Result<Settings, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut s: Settings = serde_json::from_str(&raw).map_err(|e| format!("settings: {e}"))?;
    s.overlay_scale = layout::clamp_overlay_scale(s.overlay_scale);
    s.control_size = layout::clamp_control_scale(s.control_size);
    Ok(s)
}

pub fn save(path: &Path, s: &Settings) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let pretty = serde_json::to_string_pretty(s).unwrap();
    fs::write(path, pretty + "\n").map_err(|e| e.to_string())
}

/// Propose settings: enable only AI tools that appear installed. Games stay out.
pub fn propose_from_detect(catalog: &[AgentStatus], extra_ides: &[String]) -> Settings {
    let mut s = Settings::default();
    for st in catalog {
        if st.installed {
            s.apps
                .insert(st.kind.as_str().to_string(), AppPref::default());
        }
    }
    for id in extra_ides {
        if is_ai_ide(id) {
            s.apps.insert(id.clone(), AppPref::default());
        }
    }
    s
}

pub fn is_ai_ide(id: &str) -> bool {
    matches!(
        id,
        "grok"
            | "claude"
            | "codex"
            | "antigravity"
            | "agy"
            | "pi"
            | "opencode"
            | "cursor"
            | "windsurf"
            | "zed"
            | "copilot"
            | "vscode-copilot"
    )
}

/// Merge detect proposals into existing user settings without flipping explicit disables.
pub fn merge_propose(existing: &Settings, proposed: Settings) -> Settings {
    let mut out = existing.clone();
    if out.model.is_empty() {
        out.model = proposed.model;
    }
    for (id, pref) in proposed.apps {
        out.apps.entry(id).or_insert(pref);
    }
    out
}

pub fn load_or_propose(path: &Path, proposed: Settings) -> Settings {
    let out = if path.is_file() {
        load(path)
            .map(|existing| merge_propose(&existing, proposed.clone()))
            .unwrap_or(proposed)
    } else {
        proposed
    };
    let _ = save(path, &out);
    out
}
