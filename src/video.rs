//! Decode a playable video file into overlay-sized RGBA frames via ffmpeg.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::layout::MAX_VIDEO_FRAMES;

pub fn decode_frames(path: &Path, w: u32, h: u32) -> Result<Vec<(String, Vec<u8>)>, String> {
    if !path.is_file() {
        return Err(format!("video missing {}", path.display()));
    }
    let w = w.max(1);
    let h = h.max(1);
    let tag = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let tmp = std::env::temp_dir().join(format!(
        "whipnext-vid-{}-{}",
        std::process::id(),
        slug(&tag)
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let pattern = tmp.join("f%03d.png");
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(path)
        .args([
            "-vf",
            &format!("scale={w}:{h}:flags=neighbor,fps=8"),
            "-frames:v",
            &MAX_VIDEO_FRAMES.to_string(),
            "-start_number",
            "0",
        ])
        .arg(&pattern)
        .status()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(format!("ffmpeg failed on {}", path.display()));
    }
    let mut frames = Vec::new();
    for i in 0..MAX_VIDEO_FRAMES {
        let png = tmp.join(format!("f{i:03}.png"));
        if !png.is_file() {
            break;
        }
        let img = image::open(&png).map_err(|e| e.to_string())?;
        let resized =
            image::imageops::resize(&img.to_rgba8(), w, h, image::imageops::FilterType::Triangle);
        frames.push((format!("{tag}_{i:02}"), resized.into_raw()));
    }
    let _ = std::fs::remove_dir_all(&tmp);
    if frames.is_empty() {
        return Err(format!("video '{}' produced no frames", path.display()));
    }
    Ok(frames)
}

pub fn is_playable_video(path: &Path) -> bool {
    decode_frames(path, 16, 16)
        .map(|f| !f.is_empty())
        .unwrap_or(false)
}

fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    if out.is_empty() {
        "vid".into()
    } else {
        out
    }
}

/// Tiny H.264 clip for tests: solid-color frames ffmpeg can encode.
pub fn write_test_clip(path: &Path, frames: u32, color: &str) -> Result<PathBuf, String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let n = frames.max(1);
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:s=32x32:r=8"),
            "-frames:v",
            &n.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-an",
        ])
        .arg(path)
        .status()
        .map_err(|e| format!("ffmpeg encode: {e}"))?;
    if !status.success() || !path.is_file() {
        return Err("ffmpeg encode failed".into());
    }
    Ok(path.to_path_buf())
}
