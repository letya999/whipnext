//! Swappable model + sound pack. Drop files in `assets/pack/` — no code change.
//! User-assembled characters are copied into `~/.whipnext/catalog/`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::inject::{sanitize_phrase, NEXT_PROMPT};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelManifest {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub still: String,
    #[serde(default)]
    pub idle: Vec<String>,
    #[serde(default)]
    pub action: Vec<String>,
    #[serde(default)]
    pub sound_action: Option<String>,
    #[serde(default)]
    pub sound_next: Option<String>,
    #[serde(default)]
    pub video: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phrase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub id: String,
    pub still_path: PathBuf,
    pub still_bytes: Vec<u8>,
    pub video_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pack {
    pub root: PathBuf,
    pub model: Model,
    pub sounds: BTreeMap<String, PathBuf>,
}

pub fn models_dir(root: &Path) -> PathBuf {
    root.join("models")
}

pub fn sounds_dir(root: &Path) -> PathBuf {
    root.join("sounds")
}

pub fn default_pack_root() -> PathBuf {
    crate::audio::assets_dir().join("pack")
}

pub fn catalog_dir(home: &Path) -> PathBuf {
    crate::settings::settings_dir(home).join("catalog")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Phrase {
    pub id: String,
    pub locale: String,
    pub text: String,
    #[serde(default)]
    pub sound: String,
}

pub fn list_phrases(root: &Path) -> Vec<Phrase> {
    let path = root.join("phrases.json");
    let Ok(raw) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut phrases: Vec<Phrase> = serde_json::from_str(&raw).unwrap_or_default();
    let stems = list_sound_stems(root);
    for p in &mut phrases {
        if p.sound.is_empty() && stems.iter().any(|s| s == &p.id) {
            p.sound = p.id.clone();
        }
    }
    phrases
}

/// One catalog: inject text + hit sound. Extra wavs without a line become `next`.
#[inline(never)]
pub fn list_lines(bundled: &Path, catalog: &Path) -> Vec<Phrase> {
    let mut by_id: BTreeMap<String, Phrase> = BTreeMap::new();
    for p in list_phrases(bundled) {
        by_id.insert(p.id.clone(), p);
    }
    let cat_json = catalog.join("phrases.json");
    if let Ok(raw) = fs::read_to_string(&cat_json) {
        if let Ok(extra) = serde_json::from_str::<Vec<Phrase>>(&raw) {
            for p in extra {
                by_id.insert(p.id.clone(), p);
            }
        }
    }
    let mut used: BTreeMap<String, ()> = BTreeMap::new();
    for p in by_id.values().filter(|p| !p.sound.is_empty()) {
        used.insert(p.sound.clone(), ());
    }
    for stem in list_sound_stems_merged(bundled, catalog) {
        if used.contains_key(&stem) || by_id.contains_key(&stem) {
            continue;
        }
        by_id.insert(
            stem.clone(),
            Phrase {
                id: stem.clone(),
                locale: "en".into(),
                text: NEXT_PROMPT.into(),
                sound: stem,
            },
        );
    }
    by_id.into_values().collect()
}

#[inline(never)]
pub fn import_sound_bytes(catalog: &Path, name: &str, bytes: &[u8]) -> Result<String, String> {
    if bytes.len() < 12 {
        return Err("sound too small".into());
    }
    let stem = slug_id(
        Path::new(name)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .as_ref(),
    );
    let _ = fs::create_dir_all(sounds_dir(catalog));
    let dest = sounds_dir(catalog).join(format!("{stem}.wav"));
    fs::write(&dest, bytes).map_err(|e| e.to_string())?;
    Ok(stem)
}

pub fn default_manifest(id: &str) -> ModelManifest {
    ModelManifest {
        id: id.to_string(),
        name: id.to_string(),
        still: "still.png".into(),
        idle: vec!["idle_00.png".into(), "idle_01.png".into()],
        action: vec![
            "action_00.png".into(),
            "action_01.png".into(),
            "action_02.png".into(),
            "action_03.png".into(),
        ],
        sound_action: None,
        sound_next: None,
        video: None,
        phrase: None,
        source: None,
        media_kind: None,
        folder: None,
    }
}

pub fn load_model(root: &Path, id: &str) -> Result<Model, String> {
    let dir = models_dir(root).join(id);
    let manifest = read_manifest(root, id)?;
    let still_path = dir.join(&manifest.still);
    let still_bytes = fs::read(&still_path).map_err(|e| {
        format!(
            "model '{id}' still missing at {}: {e}",
            still_path.display()
        )
    })?;
    let video_path = manifest.video.as_ref().map(|v| dir.join(v));
    Ok(Model {
        id: manifest.id,
        still_path,
        still_bytes,
        video_path: video_path.filter(|p| p.is_file()),
    })
}

pub fn load_sounds(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut map = BTreeMap::new();
    let Ok(rd) = fs::read_dir(sounds_dir(root)) else {
        return Ok(map);
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        map.insert(
            path.file_stem().unwrap().to_string_lossy().into_owned(),
            path,
        );
    }
    Ok(map)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub still: String,
    pub sound_action: String,
    pub sound_next: String,
    pub idle: Vec<String>,
    pub action: Vec<String>,
    #[serde(default)]
    pub phrase: String,
    #[serde(default)]
    pub media_kind: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub sound_file: String,
    #[serde(default)]
    pub folder_path: String,
}

pub fn read_manifest(root: &Path, id: &str) -> Result<ModelManifest, String> {
    let dir = models_dir(root).join(id);
    let manifest_path = dir.join("manifest.json");
    match fs::read_to_string(&manifest_path) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("manifest: {e}")),
        Err(_) => Ok(default_manifest(id)),
    }
}

fn is_raster_name(name: &str) -> bool {
    matches!(media_kind_of(Path::new(name)), "image" | "gif" | "svg")
}

fn model_files(dir: &Path, base: &Path, out: &mut Vec<String>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if path.is_dir() {
            model_files(&path, base, out);
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

fn manifest_is_3d(m: &ModelManifest, dir: &Path) -> bool {
    if m.media_kind.as_deref() == Some("3d") {
        return true;
    }
    if media_kind_of(Path::new(&m.still)) == "3d" {
        return true;
    }
    if m.source
        .as_deref()
        .is_some_and(|s| media_kind_of(Path::new(s)) == "3d")
    {
        return true;
    }
    dir.join("source.gltf").is_file() || dir.join("source.glb").is_file()
}

pub fn model_is_3d(root: &Path, id: &str) -> bool {
    let dir = models_dir(root).join(id);
    read_manifest(root, id)
        .map(|m| manifest_is_3d(&m, &dir))
        .unwrap_or(false)
        || gltf_source(root, id).is_some()
}

pub fn gltf_source(root: &Path, id: &str) -> Option<PathBuf> {
    let dir = models_dir(root).join(id);
    let m = read_manifest(root, id).ok();
    let mut names = Vec::new();
    if let Some(m) = &m {
        names.extend(m.source.clone());
        names.push(m.still.clone());
    }
    names.retain(|n| !n.is_empty());
    names.push("source.gltf".into());
    names.push("source.glb".into());
    for n in names {
        let p = dir.join(&n);
        if p.is_file() && media_kind_of(&p) == "3d" {
            return Some(p);
        }
    }
    let Ok(rd) = fs::read_dir(&dir) else {
        return None;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_file() && media_kind_of(&p) == "3d" {
            return Some(p);
        }
    }
    None
}

fn info_from_manifest(root: &Path, id: &str, m: ModelManifest) -> ModelInfo {
    let still_name = if m.still.is_empty() {
        "still.png".into()
    } else {
        m.still.clone()
    };
    let dir = models_dir(root).join(id);
    let mut files = Vec::new();
    model_files(&dir, &dir, &mut files);
    files.sort();
    let sound_action = m.sound_action.clone().unwrap_or_else(|| "whip".into());
    let kind = m
        .media_kind
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let k = media_kind_of(Path::new(&still_name));
            if k == "file" {
                "image".into()
            } else {
                k.into()
            }
        });
    let preview_name = if dir.join("preview.png").is_file() {
        "preview.png".to_string()
    } else {
        still_name.clone()
    };
    let still = if kind == "3d" && !is_raster_name(&preview_name) {
        String::new()
    } else {
        format!("models/{id}/{preview_name}")
    };
    ModelInfo {
        id: m.id.clone(),
        name: if m.name.is_empty() {
            m.id.clone()
        } else {
            m.name.clone()
        },
        still,
        sound_action: sound_action.clone(),
        sound_next: m.sound_next.unwrap_or_else(|| "next".into()),
        idle: m.idle,
        action: m.action,
        phrase: m.phrase.unwrap_or_else(|| NEXT_PROMPT.into()),
        media_kind: kind,
        source: m
            .source
            .filter(|s| !s.is_empty())
            .map(|s| format!("models/{id}/{s}"))
            .unwrap_or_default(),
        folder: m.folder.unwrap_or_else(|| format!("models/{id}")),
        files,
        sound_file: if root
            .join("sounds")
            .join(format!("{sound_action}.wav"))
            .is_file()
        {
            format!("sounds/{sound_action}.wav")
        } else {
            String::new()
        },
        folder_path: dir.display().to_string(),
    }
}

pub fn list_models(root: &Path) -> Vec<ModelInfo> {
    list_model_ids(root)
        .into_iter()
        .filter_map(|id| {
            let m = read_manifest(root, &id).ok()?;
            Some(info_from_manifest(root, &id, m))
        })
        .collect()
}

/// Catalog entries override bundled models with the same id.
pub fn list_models_merged(bundled: &Path, catalog: &Path) -> Vec<ModelInfo> {
    let mut by_id: BTreeMap<String, ModelInfo> = BTreeMap::new();
    for m in list_models(bundled) {
        by_id.insert(m.id.clone(), m);
    }
    for m in list_models(catalog) {
        by_id.insert(m.id.clone(), m);
    }
    by_id.into_values().collect()
}

pub fn list_sound_stems_merged(bundled: &Path, catalog: &Path) -> Vec<String> {
    let mut names: BTreeMap<String, ()> = BTreeMap::new();
    for n in list_sound_stems(bundled) {
        names.insert(n, ());
    }
    for n in list_sound_stems(catalog) {
        names.insert(n, ());
    }
    names.into_keys().collect()
}

pub fn resolve_pack_root(bundled: &Path, catalog: &Path, id: &str) -> PathBuf {
    let cat = models_dir(catalog).join(id);
    if cat.is_dir() {
        catalog.to_path_buf()
    } else {
        bundled.to_path_buf()
    }
}

pub fn frame_paths(root: &Path, id: &str) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
    let dir = models_dir(root).join(id);
    let m = read_manifest(root, id)?;
    if manifest_is_3d(&m, &dir) {
        let src = gltf_source(root, id).ok_or_else(|| format!("model '{id}' has no glTF"))?;
        return Ok((vec![src.clone()], vec![src]));
    }
    let idle: Vec<PathBuf> = m
        .idle
        .iter()
        .map(|n| dir.join(n))
        .filter(|p| p.is_file())
        .collect();
    let action: Vec<PathBuf> = m
        .action
        .iter()
        .map(|n| dir.join(n))
        .filter(|p| p.is_file())
        .collect();
    if idle.is_empty() {
        return Err(format!("model '{id}' has no idle frames"));
    }
    let gif_action = action.len() == 1 && matches!(media_kind_of(&action[0]), "gif" | "video");
    if action.len() < crate::layout::MIN_ACTION_FRAMES && !gif_action {
        return Err(format!("model '{id}' needs at least 2 action frames"));
    }
    Ok((idle, action))
}

pub fn list_model_ids(root: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    let dir = models_dir(root);
    let Ok(rd) = fs::read_dir(dir) else {
        return ids;
    };
    for ent in rd.flatten() {
        if !ent.path().is_dir() {
            continue;
        }
        ids.push(ent.file_name().to_string_lossy().into_owned());
    }
    ids.sort();
    ids
}

pub fn list_sound_stems(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = load_sounds(root).unwrap_or_default().into_keys().collect();
    names.sort();
    names
}

#[inline(never)]
pub fn load_pack(root: &Path, model_id: &str) -> Result<Pack, String> {
    Ok(Pack {
        root: root.to_path_buf(),
        model: load_model(root, model_id)?,
        sounds: load_sounds(root).unwrap_or_default(),
    })
}

pub fn sound_bytes(pack: &Pack, name: &str) -> Result<Vec<u8>, String> {
    let path = pack
        .sounds
        .get(name)
        .ok_or_else(|| format!("sound '{name}' not in pack"))?;
    fs::read(path).map_err(|e| e.to_string())
}

pub fn sound_action_from_manifest(root: &Path, id: &str) -> Option<String> {
    read_manifest(root, id)
        .ok()
        .and_then(|m| m.sound_action)
        .filter(|s| !s.is_empty())
}

pub fn sound_next_from_manifest(root: &Path, id: &str) -> Option<String> {
    read_manifest(root, id)
        .ok()
        .and_then(|m| m.sound_next)
        .filter(|s| !s.is_empty())
}

pub fn phrase_from_manifest(root: &Path, id: &str) -> String {
    read_manifest(root, id)
        .ok()
        .and_then(|m| m.phrase)
        .map(|p| sanitize_phrase(&p))
        .unwrap_or_else(|| NEXT_PROMPT.to_string())
}

pub fn slug_id(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if (c.is_whitespace() || c == '-' || c == '_')
            && !out.ends_with('-')
            && !out.is_empty()
        {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "character".into()
    } else {
        out
    }
}

pub fn unique_id(root: &Path, base: &str) -> String {
    if !models_dir(root).join(base).exists() {
        return base.to_string();
    }
    let mut n = 2u32;
    loop {
        let id = format!("{base}-{n}");
        if !models_dir(root).join(&id).exists() {
            return id;
        }
        n += 1;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportSpec {
    pub name: String,
    pub phrase: Option<String>,
    /// Single clip used for all three states when per-state files are absent.
    pub media: Option<PathBuf>,
    pub still: Option<PathBuf>,
    pub idle: Option<PathBuf>,
    pub action: Option<PathBuf>,
    pub hit_sound: Option<PathBuf>,
    pub idle_set: Vec<PathBuf>,
    pub action_set: Vec<PathBuf>,
    pub extra_files: Vec<(PathBuf, PathBuf)>,
}

pub fn media_kind_of(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "gif" => "gif",
        "svg" | "svgz" => "svg",
        "gltf" | "glb" => "3d",
        "mp4" | "mov" | "webm" | "avi" => "video",
        "png" | "jpg" | "jpeg" | "bmp" | "webp" => "image",
        "wav" => "sound",
        "obj" | "fbx" | "vrm" | "blend" | "mp3" | "ogg" => "unsupported",
        _ => "file",
    }
}

pub fn playable_clip_kind(path: &Path) -> Result<&'static str, String> {
    match media_kind_of(path) {
        "file" | "unsupported" | "sound" => Err(format!(
            "format not playable: {}",
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or(path.file_name().and_then(|n| n.to_str()).unwrap_or("file"))
        )),
        kind => Ok(kind),
    }
}

fn ext_of(path: &Path, fallback: &str) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.into())
}

#[inline(never)]
fn copy_file(src: &Path, dest: &Path) -> Result<(), String> {
    let _ = dest.parent().map(fs::create_dir_all);
    fs::copy(src, dest).map_err(|e| format!("copy {}: {e}", src.display()))?;
    Ok(())
}

fn copy_dir(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let rd = fs::read_dir(src).map_err(|e| e.to_string())?;
    for ent in rd.flatten() {
        let from = ent.path();
        let to = dest.join(ent.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else if from.is_file() {
            copy_file(&from, &to)?;
        }
    }
    Ok(())
}

pub fn ensure_catalog_character(bundled: &Path, catalog: &Path, id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("invalid character id".into());
    }
    let dest = models_dir(catalog).join(id);
    if dest.is_dir() {
        return Ok(());
    }
    let source = models_dir(bundled).join(id);
    if !source.is_dir() {
        return Err(format!("character '{id}' is not available"));
    }
    copy_dir(&source, &dest)
}

#[inline(never)]
fn copy_hit_sound(catalog: &Path, _id: &str, snd: &Path) -> Option<String> {
    if !snd.is_file() {
        return None;
    }
    let stem = slug_id(&snd.file_stem().unwrap_or_default().to_string_lossy());
    let dest = sounds_dir(catalog).join(format!("{stem}.wav"));
    copy_file(snd, &dest).ok()?;
    Some(stem)
}

#[inline(never)]
fn write_placeholder_png(path: &Path) {
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 210, 122, 255]));
    let _ = img.save(path);
}

#[inline(never)]
fn raster_or_placeholder(src: &Path, dest_png: &Path) -> bool {
    let ok = image::open(src)
        .ok()
        .and_then(|img| img.to_rgba8().save(dest_png).ok());
    if ok.is_some() {
        return true;
    }
    write_placeholder_png(dest_png);
    false
}

fn duplicate_named(src: &Path, dest_dir: &Path, names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        let dest = dest_dir.join(name);
        let _ = copy_file(src, &dest);
        out.push((*name).to_string());
    }
    out
}

fn first_clip(spec: &ImportSpec) -> Option<PathBuf> {
    spec.still
        .clone()
        .or(spec.media.clone())
        .or(spec.idle.clone())
        .or(spec.action.clone())
        .or_else(|| spec.idle_set.first().cloned())
        .or_else(|| spec.action_set.first().cloned())
}

fn spec_has_media(spec: &ImportSpec) -> bool {
    first_clip(spec).is_some()
}

fn write_manifest(dir: &Path, manifest: &ModelManifest) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    fs::write(dir.join("manifest.json"), raw + "\n").map_err(|e| e.to_string())
}

fn idle_sources(spec: &ImportSpec, clip: &Path) -> Vec<PathBuf> {
    if !spec.idle_set.is_empty() {
        spec.idle_set.clone()
    } else if let Some(p) = spec.idle.clone() {
        vec![p]
    } else {
        vec![clip.to_path_buf()]
    }
}

fn action_sources(spec: &ImportSpec, clip: &Path) -> Vec<PathBuf> {
    if !spec.action_set.is_empty() {
        spec.action_set.clone()
    } else if let Some(p) = spec.action.clone() {
        vec![p]
    } else {
        vec![clip.to_path_buf()]
    }
}

fn copy_clip_set(files: &[PathBuf], dest: &Path, stem: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for (i, src) in files.iter().enumerate() {
        if !src.is_file() {
            return Err(format!("missing file {}", src.display()));
        }
        let kind = playable_clip_kind(src)?;
        if kind == "video" && !crate::video::is_playable_video(src) {
            return Err("format not playable: video".into());
        }
        let ext = ext_of(src, "png");
        let name = if files.len() == 1 {
            format!("{stem}.{ext}")
        } else {
            format!("{stem}_{i:02}.{ext}")
        };
        copy_file(src, &dest.join(&name))?;
        names.push(name);
    }
    Ok(names)
}

fn copy_extra_files(files: &[(PathBuf, PathBuf)], dest: &Path) -> Result<(), String> {
    for (source, relative) in files {
        copy_file(source, &dest.join("_source").join(relative))?;
    }
    Ok(())
}

fn prune_source_dir(dir: &Path, base: &Path, keep: &BTreeMap<String, ()>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            prune_source_dir(&path, base, keep);
            let _ = fs::remove_dir(&path);
        } else if path.is_file() {
            let Some(relative) = path
                .strip_prefix(base)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
            else {
                continue;
            };
            if !keep.contains_key(&relative) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

fn prune_source_files(dir: &Path, files: &[(PathBuf, PathBuf)]) {
    let mut keep = BTreeMap::new();
    for (_, relative) in files {
        keep.insert(relative.to_string_lossy().replace('\\', "/"), ());
    }
    prune_source_dir(&dir.join("_source"), &dir.join("_source"), &keep);
}

fn write_model_files(
    catalog: &Path,
    id: &str,
    spec: &ImportSpec,
    name: &str,
) -> Result<ModelManifest, String> {
    let dir = models_dir(catalog).join(id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let _ = fs::create_dir_all(sounds_dir(catalog));
    copy_extra_files(&spec.extra_files, &dir)?;
    let clip = first_clip(spec)
        .ok_or_else(|| "pick a 2D/3D/GIF/SVG/video clip or state images".to_string())?;
    if !clip.is_file() {
        return Err(format!("missing file {}", clip.display()));
    }
    let kind = playable_clip_kind(&clip)?;
    if kind == "video" && !crate::video::is_playable_video(&clip) {
        return Err("format not playable: video".into());
    }
    let still_src = spec.still.as_ref().unwrap_or(&clip);
    let ext = ext_of(still_src, "png");
    let original_name = format!("source.{ext}");
    copy_file(still_src, &dir.join(&original_name))?;
    for side in crate::gltf::gltf_sidecars(still_src) {
        let _ = copy_file(&side, &dir.join(side.file_name().unwrap_or_default()));
    }
    let sound_action = spec
        .hit_sound
        .as_ref()
        .and_then(|snd| copy_hit_sound(catalog, id, snd));
    let phrase = spec.phrase.as_deref().map(sanitize_phrase);

    if kind == "3d" {
        let manifest = ModelManifest {
            id: id.to_string(),
            name: name.to_string(),
            still: original_name.clone(),
            idle: vec![],
            action: vec![],
            sound_action,
            sound_next: Some("next".into()),
            video: None,
            phrase,
            source: Some(original_name),
            media_kind: Some(kind.into()),
            folder: Some(format!("models/{id}")),
        };
        write_manifest(&dir, &manifest)?;
        prune_source_files(&dir, &spec.extra_files);
        return Ok(manifest);
    }

    if kind == "video" {
        let idle = copy_clip_set(&idle_sources(spec, &clip), &dir, "idle")?;
        let action = copy_clip_set(&action_sources(spec, &clip), &dir, "action")?;
        if let Ok(frames) = crate::video::decode_frames(&clip, 64, 64) {
            if let Some((_, raw)) = frames.first() {
                if let Some(img) = image::RgbaImage::from_raw(64, 64, raw.clone()) {
                    let _ = img.save(dir.join("still.png"));
                }
            }
        }
        let still = if dir.join("still.png").is_file() {
            "still.png".into()
        } else {
            original_name.clone()
        };
        let manifest = ModelManifest {
            id: id.to_string(),
            name: name.to_string(),
            still,
            idle,
            action,
            sound_action,
            sound_next: Some("next".into()),
            video: Some(original_name.clone()),
            phrase,
            source: Some(original_name),
            media_kind: Some(kind.into()),
            folder: Some(format!("models/{id}")),
        };
        write_manifest(&dir, &manifest)?;
        prune_source_files(&dir, &spec.extra_files);
        return Ok(manifest);
    }

    let still_name = "still.png".to_string();
    let decoded = raster_or_placeholder(still_src, &dir.join(&still_name));
    let still_file = if decoded {
        still_name.clone()
    } else if kind == "image" || kind == "gif" {
        original_name.clone()
    } else {
        still_name.clone()
    };

    let idle_src_list = idle_sources(spec, still_src);
    let action_src_list = action_sources(spec, still_src);
    let idle = if idle_src_list.len() > 1
        || idle_src_list
            .first()
            .is_some_and(|p| matches!(media_kind_of(p), "gif" | "video"))
    {
        copy_clip_set(&idle_src_list, &dir, "idle")?
    } else {
        let idle_src = idle_src_list.first().unwrap_or(still_src);
        let idle_png = dir.join("_idle.png");
        let _ = raster_or_placeholder(idle_src, &idle_png);
        let names = duplicate_named(&idle_png, &dir, &["idle_00.png", "idle_01.png"]);
        let _ = fs::remove_file(&idle_png);
        names
    };
    let action = if action_src_list.len() > 1
        || action_src_list
            .first()
            .is_some_and(|p| matches!(media_kind_of(p), "gif" | "video"))
    {
        copy_clip_set(&action_src_list, &dir, "action")?
    } else {
        let action_src = action_src_list.first().unwrap_or(still_src);
        let action_png = dir.join("_action.png");
        let _ = raster_or_placeholder(action_src, &action_png);
        let names = duplicate_named(
            &action_png,
            &dir,
            &[
                "action_00.png",
                "action_01.png",
                "action_02.png",
                "action_03.png",
            ],
        );
        let _ = fs::remove_file(&action_png);
        names
    };

    let manifest = ModelManifest {
        id: id.to_string(),
        name: name.to_string(),
        still: still_file,
        idle,
        action,
        sound_action,
        sound_next: Some("next".into()),
        video: None,
        phrase,
        source: Some(original_name),
        media_kind: Some(kind.into()),
        folder: Some(format!("models/{id}")),
    };
    write_manifest(&dir, &manifest)?;
    prune_source_files(&dir, &spec.extra_files);
    Ok(manifest)
}

/// Copy picker files into the app catalog and write `manifest.json` so a later
/// load uses only catalog paths.
#[inline(never)]
pub fn import_character(catalog: &Path, spec: &ImportSpec) -> Result<ModelManifest, String> {
    if spec.name.trim().is_empty() {
        return Err("character needs a name".into());
    }
    let id = unique_id(catalog, &slug_id(&spec.name));
    write_model_files(catalog, &id, spec, spec.name.trim())
}

pub fn rename_character(catalog: &Path, id: &str, name: &str) -> Result<ModelManifest, String> {
    if id.trim().is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
        || name.trim().is_empty()
    {
        return Err("character name required".into());
    }
    let dir = models_dir(catalog).join(id);
    if !dir.is_dir() {
        return Err(format!("character '{id}' is not in the catalog"));
    }
    let mut manifest = read_manifest(catalog, id)?;
    manifest.name = name.trim().to_string();
    manifest.folder = Some(format!("models/{id}"));
    write_manifest(&dir, &manifest)?;
    Ok(manifest)
}

/// Replace still/idle/action files and/or the hit sound of an existing catalog character.
#[inline(never)]
pub fn replace_character(
    catalog: &Path,
    id: &str,
    spec: &ImportSpec,
) -> Result<ModelManifest, String> {
    if id.trim().is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("character id required".into());
    }
    let dir = models_dir(catalog).join(id);
    if !dir.is_dir() {
        return Err(format!("character '{id}' is not in the catalog"));
    }
    let mut manifest = read_manifest(catalog, id)?;
    let prev_sound = manifest.sound_action.clone();
    let prev_phrase = manifest.phrase.clone();
    if spec_has_media(spec) {
        let name = if spec.name.trim().is_empty() {
            if manifest.name.is_empty() {
                id.to_string()
            } else {
                manifest.name.clone()
            }
        } else {
            spec.name.trim().to_string()
        };
        manifest = write_model_files(catalog, id, spec, &name)?;
        if spec.hit_sound.is_none() {
            manifest.sound_action = prev_sound.clone();
        }
        if spec.phrase.is_none() {
            manifest.phrase = prev_phrase.clone();
        }
        write_manifest(&dir, &manifest)?;
    }
    if let Some(snd) = &spec.hit_sound {
        manifest.sound_action = copy_hit_sound(catalog, id, snd);
        write_manifest(&dir, &manifest)?;
    }
    if let Some(p) = &spec.phrase {
        manifest.phrase = Some(sanitize_phrase(p));
        write_manifest(&dir, &manifest)?;
    }
    if !spec_has_media(spec) && spec.hit_sound.is_none() && spec.phrase.is_none() {
        return Err("pick images, video, or a hit sound to replace".into());
    }
    Ok(manifest)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportBytesFile {
    pub role: String,
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Write in-memory picker bytes to a staging dir, then import (catalog copies).
pub fn import_character_bytes(
    catalog: &Path,
    name: &str,
    phrase: Option<&str>,
    files: &[ImportBytesFile],
) -> Result<ModelManifest, String> {
    let stage = catalog.join(".stage").join(slug_id(name));
    let _ = fs::remove_dir_all(&stage);
    let _ = fs::create_dir_all(&stage);
    let spec = spec_from_bytes(name, phrase, files, &stage);
    let out = import_character(catalog, &spec);
    let _ = fs::remove_dir_all(&stage);
    out
}

fn spec_from_bytes(
    name: &str,
    phrase: Option<&str>,
    files: &[ImportBytesFile],
    stage: &Path,
) -> ImportSpec {
    let mut spec = ImportSpec {
        name: name.to_string(),
        phrase: phrase.map(|s| s.to_string()),
        ..ImportSpec::default()
    };
    for f in files {
        if f.name.is_empty() || f.bytes.is_empty() {
            continue;
        }
        let Some(relative) = safe_relative_path(&f.name) else {
            continue;
        };
        let path = stage.join(&relative);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, &f.bytes);
        spec.extra_files.push((path.clone(), relative));
        let kind = media_kind_of(&path);
        match f.role.as_str() {
            "idle" => {
                spec.idle = Some(path.clone());
                spec.idle_set.push(path);
            }
            "action" => {
                spec.action = Some(path.clone());
                spec.action_set.push(path);
            }
            "sound" | "hit" | "hit_sound" => spec.hit_sound = Some(path),
            "bin" | "buffer" => {}
            _ => assign_import_clip(&mut spec, path, kind),
        }
    }
    spec
}

fn safe_relative_path(raw: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in Path::new(raw).components() {
        match part {
            Component::Normal(value) => out.push(value),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return None,
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

/// Replace media/sound of an existing catalog id from in-memory picker bytes.
pub fn replace_character_bytes(
    catalog: &Path,
    id: &str,
    phrase: Option<&str>,
    files: &[ImportBytesFile],
) -> Result<ModelManifest, String> {
    let stage = catalog
        .join(".stage")
        .join(format!("replace-{}", slug_id(id)));
    let _ = fs::remove_dir_all(&stage);
    let _ = fs::create_dir_all(&stage);
    let spec = spec_from_bytes(id, phrase, files, &stage);
    let out = replace_character(catalog, id, &spec);
    let _ = fs::remove_dir_all(&stage);
    out
}

fn clip_rank(kind: &str) -> u8 {
    match kind {
        "3d" => 3,
        "gif" | "svg" | "video" => 2,
        "image" => 1,
        _ => 0,
    }
}

fn assign_import_clip(spec: &mut ImportSpec, path: PathBuf, kind: &str) {
    if kind == "sound" {
        spec.hit_sound = Some(path);
        return;
    }
    if clip_rank(kind) == 0 {
        return;
    }
    let cur = spec.media.as_ref().or(spec.still.as_ref());
    let cur_rank = cur.map(|p| clip_rank(media_kind_of(p))).unwrap_or(0);
    if clip_rank(kind) >= cur_rank {
        spec.media = Some(path);
    }
}
