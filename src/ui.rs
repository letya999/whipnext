//! Settings snapshot + asset serving + IPC parse. No window toolkit.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::json;

use crate::detect::{self, Probe};
use crate::focus::surface_for_exe;
use crate::pack::{self, ImportBytesFile, ImportSpec};
use crate::settings::{self, Settings};

pub fn proposed_from(probe: &dyn Probe) -> Settings {
    let cat = detect::catalog(probe);
    let extra: Vec<String> = probe
        .running()
        .into_iter()
        .filter_map(|(_, n)| surface_for_exe(&n).map(|s| s.to_string()))
        .filter(|id| settings::is_ai_ide(id))
        .collect();
    settings::propose_from_detect(&cat, &extra)
}

fn model_json(m: &pack::ModelInfo) -> serde_json::Value {
    let three = m.media_kind == "3d";
    json!({
        "id": m.id,
        "name": m.name,
        "preview": if m.still.is_empty() {
            String::new()
        } else {
            format!("/pack/{}", m.still.replace('\\', "/"))
        },
        "sound_action": m.sound_action,
        "sound_next": m.sound_next,
        "idle": if three {
            m.idle.clone()
        } else {
            m.idle.iter().map(|f| format!("/pack/models/{}/{f}", m.id)).collect::<Vec<_>>()
        },
        "action": if three {
            m.action.clone()
        } else {
            m.action.iter().map(|f| format!("/pack/models/{}/{f}", m.id)).collect::<Vec<_>>()
        },
        "phrase": m.phrase,
        "media_kind": m.media_kind,
        "source": if m.source.is_empty() {
            String::new()
        } else {
            format!("/pack/{}", m.source.replace('\\', "/"))
        },
        "folder": m.folder,
        "files": m.files.iter().map(|f| format!("/pack/models/{}/{}", m.id, f)).collect::<Vec<_>>(),
        "sound_file": if m.sound_file.is_empty() {
            String::new()
        } else {
            format!("/pack/{}", m.sound_file.replace('\\', "/"))
        },
        "folder_path": m.folder_path,
    })
}

pub fn snapshot(settings: &Settings, pack_root: &Path) -> serde_json::Value {
    snapshot_merged(settings, pack_root, Path::new(""))
}

pub fn snapshot_merged(settings: &Settings, bundled: &Path, catalog: &Path) -> serde_json::Value {
    let models: Vec<_> = pack::list_models_merged(bundled, catalog)
        .into_iter()
        .map(|m| model_json(&m))
        .collect();
    let herdr: Vec<_> = crate::live::list_herdr_agents()
        .into_iter()
        .map(|a| {
            json!({
                "kind": a.kind.as_str(),
                "name": a.name,
                "pane_id": a.pane_id,
                "focused": a.focused,
                "session_id": a.session_id,
            })
        })
        .collect();
    let phrases: Vec<_> = pack::list_lines(bundled, catalog)
        .into_iter()
        .map(|p| {
            json!({
                "id": p.id,
                "locale": p.locale,
                "text": p.text,
                "sound": p.sound,
            })
        })
        .collect();
    let catalog_models = catalog.join("models");
    let catalog_sounds = catalog.join("sounds");
    let bundled_models = bundled.join("models");
    let bundled_sounds = bundled.join("sounds");
    let sound_files: Vec<_> = pack::list_sound_stems_merged(bundled, catalog)
        .into_iter()
        .map(|stem| {
            let catalog_file = catalog_sounds.join(format!("{stem}.wav"));
            let path = if catalog_file.is_file() {
                catalog_file
            } else {
                bundled_sounds.join(format!("{stem}.wav"))
            };
            json!({ "stem": stem, "path": path.display().to_string() })
        })
        .collect();
    json!({
        "settings": settings,
        "models": models,
        "sounds": pack::list_sound_stems_merged(bundled, catalog),
        "sound_files": sound_files,
        "phrases": phrases,
        "logo": "/icon-256.png",
        "catalog": catalog.display().to_string(),
        "folders": {
            "catalog_models": catalog_models.display().to_string(),
            "catalog_sounds": catalog_sounds.display().to_string(),
            "bundled_models": bundled_models.display().to_string(),
            "bundled_sounds": bundled_sounds.display().to_string(),
        },
        "herdr": herdr,
        "layout": crate::layout::snapshot(),
    })
}

#[inline(never)]
pub fn apply_settings(path: &Path, live: &Mutex<Settings>, incoming: Settings) -> Settings {
    let _ = settings::save(path, &incoming);
    if let Ok(mut g) = live.lock() {
        *g = incoming.clone();
    }
    incoming
}

pub fn catalog_from_settings_path(path: &Path) -> PathBuf {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join("catalog"),
        _ => PathBuf::from("catalog"),
    }
}

pub fn apply_imported(
    path: &Path,
    live: &Mutex<Settings>,
    manifest: pack::ModelManifest,
) -> Settings {
    apply_character_settings(path, live, manifest, true)
}

pub fn apply_character_settings(
    path: &Path,
    live: &Mutex<Settings>,
    manifest: pack::ModelManifest,
    make_default: bool,
) -> Settings {
    let mut cur = live.lock().ok().map(|g| g.clone()).unwrap_or_default();
    if make_default {
        cur.model = manifest.id.clone();
    }
    if let Some(p) = &manifest.phrase {
        cur.phrases.insert(manifest.id.clone(), p.clone());
    }
    if let Some(s) = &manifest.sound_action {
        cur.model_sounds.insert(manifest.id.clone(), s.clone());
    }
    apply_settings(path, live, cur)
}

pub fn launch_log_path(home: &Path) -> PathBuf {
    settings::settings_dir(home).join("launch.log")
}

#[inline(never)]
pub fn write_launch_log(home: &Path, msg: &str) {
    let p = launch_log_path(home);
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
        .and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "{msg}")
        });
}

pub fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "ico" => "image/x-icon",
        "gltf" => "model/gltf+json",
        "glb" => "model/gltf-binary",
        _ => "application/octet-stream",
    }
}

pub struct AssetBody {
    pub status: u16,
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

pub fn serve_asset(assets: &Path, uri_path: &str) -> AssetBody {
    serve_asset_ex(assets, Path::new(""), uri_path)
}

pub fn serve_asset_ex(assets: &Path, catalog: &Path, uri_path: &str) -> AssetBody {
    let rel = uri_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "ui/index.html" } else { rel };
    if rel.contains("..") {
        return AssetBody {
            status: 400,
            mime: "application/octet-stream",
            bytes: Vec::new(),
        };
    }
    let file = assets.join(rel);
    let mut bytes = std::fs::read(&file).unwrap_or_default();
    let mut used = file.clone();
    if (bytes.is_empty() && !file.is_file()) || rel.starts_with("pack/") {
        let cat_rel = rel.strip_prefix("pack/").unwrap_or(rel);
        let cat_file = catalog.join(cat_rel);
        if cat_file.is_file() {
            if let Ok(b) = std::fs::read(&cat_file) {
                bytes = b;
                used = cat_file;
            }
        }
    }
    let status = if bytes.is_empty() && !used.is_file() {
        404
    } else {
        200
    };
    AssetBody {
        status,
        mime: mime(&used),
        bytes,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ipc {
    Ready,
    Set(Settings),
    Redetect,
    Quit,
    Play(String),
    Import(ImportSpec),
    ImportBytes {
        name: String,
        phrase: Option<String>,
        files: Vec<ImportBytesFile>,
    },
    ImportSound {
        name: String,
        b64: String,
    },
    ReplaceBytes {
        id: String,
        phrase: Option<String>,
        files: Vec<ImportBytesFile>,
    },
    Rename {
        id: String,
        name: String,
    },
    OpenCharacterFolder(String),
    OpenSoundFolder,
    Ignore,
}

fn path_field(v: &serde_json::Value, key: &str) -> Option<PathBuf> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

#[inline(never)]
pub fn decode_b64(input: &str) -> Result<Vec<u8>, String> {
    let s: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if !s.len().is_multiple_of(4) {
        return Err("base64 length".into());
    }
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let pad = (bytes[i + 2] == b'=') as usize + (bytes[i + 3] == b'=') as usize;
        let a = val(bytes[i]).ok_or_else(|| "base64".to_string())?;
        let b = val(bytes[i + 1]).ok_or_else(|| "base64".to_string())?;
        let c = if pad >= 2 {
            0
        } else {
            val(bytes[i + 2]).ok_or_else(|| "base64".to_string())?
        };
        let d = if pad >= 1 {
            0
        } else {
            val(bytes[i + 3]).ok_or_else(|| "base64".to_string())?
        };
        out.push((a << 2) | (b >> 4));
        if pad < 2 {
            out.push((b << 4) | (c >> 2));
        }
        if pad < 1 {
            out.push((c << 6) | d);
        }
        i += 4;
    }
    Ok(out)
}

#[inline(never)]
fn parse_import_bytes(v: &serde_json::Value) -> Option<Ipc> {
    let name = v.get("name").and_then(|x| x.as_str())?.to_string();
    if name.trim().is_empty() {
        return None;
    }
    let phrase = v
        .get("phrase")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let mut files = Vec::new();
    if let Some(arr) = v.get("files").and_then(|x| x.as_array()) {
        for f in arr {
            let role = f.get("role").and_then(|x| x.as_str()).unwrap_or("still");
            let fname = f.get("name").and_then(|x| x.as_str()).unwrap_or("file.bin");
            let b64 = f.get("b64").and_then(|x| x.as_str()).unwrap_or("");
            let Ok(bytes) = decode_b64(b64) else {
                continue;
            };
            files.push(ImportBytesFile {
                role: role.into(),
                name: fname.into(),
                bytes,
            });
        }
    }
    Some(Ipc::ImportBytes {
        name,
        phrase,
        files,
    })
}

fn parse_replace_bytes(v: &serde_json::Value) -> Option<Ipc> {
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return None;
    }
    let mut obj = v.clone();
    if obj
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        obj["name"] = json!(id.clone());
    }
    match parse_import_bytes(&obj) {
        Some(Ipc::ImportBytes { phrase, files, .. }) => {
            Some(Ipc::ReplaceBytes { id, phrase, files })
        }
        _ => None,
    }
}

pub fn parse_ipc(msg: &str) -> Ipc {
    let v: serde_json::Value = serde_json::from_str(msg).unwrap_or(json!({}));
    match v.get("cmd").and_then(|c| c.as_str()).unwrap_or("") {
        "ready" => Ipc::Ready,
        "set" => match serde_json::from_value::<Settings>(v["settings"].clone()) {
            Ok(s) => Ipc::Set(s),
            Err(_) => Ipc::Ignore,
        },
        "redetect" => Ipc::Redetect,
        "quit" => Ipc::Quit,
        "play" => v
            .get("sound")
            .and_then(|x| x.as_str())
            .map(|s| Ipc::Play(s.to_string()))
            .unwrap_or(Ipc::Ignore),
        "import" => {
            let name = v
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if name.trim().is_empty() {
                return Ipc::Ignore;
            }
            Ipc::Import(ImportSpec {
                name,
                phrase: v
                    .get("phrase")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string()),
                media: path_field(&v, "media"),
                still: path_field(&v, "still"),
                idle: path_field(&v, "idle"),
                action: path_field(&v, "action"),
                hit_sound: path_field(&v, "sound").or_else(|| path_field(&v, "hit_sound")),
                idle_set: Vec::new(),
                action_set: Vec::new(),
                extra_files: Vec::new(),
            })
        }
        "import-bytes" => parse_import_bytes(&v).unwrap_or(Ipc::Ignore),
        "replace-bytes" => parse_replace_bytes(&v).unwrap_or(Ipc::Ignore),
        "rename" => {
            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").trim();
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").trim();
            if id.is_empty() || name.is_empty() {
                Ipc::Ignore
            } else {
                Ipc::Rename {
                    id: id.into(),
                    name: name.into(),
                }
            }
        }
        "open-character-folder" => v
            .get("id")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty() && !s.contains(['/', '\\']))
            .map(|s| Ipc::OpenCharacterFolder(s.into()))
            .unwrap_or(Ipc::Ignore),
        "open-sound-folder" => Ipc::OpenSoundFolder,
        "import-sound" => {
            let name = v
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let b64 = v
                .get("b64")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if name.trim().is_empty() || b64.is_empty() {
                Ipc::Ignore
            } else {
                Ipc::ImportSound { name, b64 }
            }
        }
        _ => Ipc::Ignore,
    }
}

#[inline(never)]
pub fn apply_ipc(
    ipc: Ipc,
    path: &Path,
    live: &Mutex<Settings>,
    proposed: Settings,
) -> Option<Settings> {
    match ipc {
        Ipc::Set(s) => {
            apply_settings(path, live, s);
            None
        }
        Ipc::Redetect => {
            let cur = live.lock().ok().map(|g| g.clone()).unwrap_or_default();
            Some(apply_settings(
                path,
                live,
                settings::merge_propose(&cur, proposed),
            ))
        }
        Ipc::Ready => None,
        Ipc::Import(spec) => {
            let catalog = catalog_from_settings_path(path);
            match pack::import_character(&catalog, &spec) {
                Ok(m) => Some(apply_imported(path, live, m)),
                Err(_) => live.lock().ok().map(|g| g.clone()),
            }
        }
        Ipc::ImportBytes {
            name,
            phrase,
            files,
        } => {
            let catalog = catalog_from_settings_path(path);
            match pack::import_character_bytes(&catalog, &name, phrase.as_deref(), &files) {
                Ok(m) => Some(apply_imported(path, live, m)),
                Err(_) => live.lock().ok().map(|g| g.clone()),
            }
        }
        Ipc::ReplaceBytes { id, phrase, files } => {
            let catalog = catalog_from_settings_path(path);
            let _ = pack::ensure_catalog_character(&pack::default_pack_root(), &catalog, &id);
            match pack::replace_character_bytes(&catalog, &id, phrase.as_deref(), &files) {
                Ok(m) => Some(apply_character_settings(path, live, m, false)),
                Err(_) => live.lock().ok().map(|g| g.clone()),
            }
        }
        Ipc::Rename { id, name } => {
            let catalog = catalog_from_settings_path(path);
            let _ = pack::ensure_catalog_character(&pack::default_pack_root(), &catalog, &id);
            let _ = pack::rename_character(&catalog, &id, &name);
            None
        }
        Ipc::ImportSound { name, b64 } => {
            let catalog = catalog_from_settings_path(path);
            let Ok(bytes) = decode_b64(&b64) else {
                return live.lock().ok().map(|g| g.clone());
            };
            match pack::import_sound_bytes(&catalog, &name, &bytes) {
                Ok(stem) => {
                    let mut cur = live.lock().ok().map(|g| g.clone()).unwrap_or_default();
                    let model = cur.model.clone();
                    cur.model_sounds.insert(model, stem);
                    Some(apply_settings(path, live, cur))
                }
                Err(_) => live.lock().ok().map(|g| g.clone()),
            }
        }
        _ => None,
    }
}
