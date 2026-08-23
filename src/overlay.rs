//! Overlay coordinator: skin + whip machine + host. Window FFI stays in the host.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use crate::audio::OsAudio;
use crate::focus::{self, Foreground};
use crate::inject::RecordingInjector;
use crate::layout::{self, ALPHA_HIT, OVERLAY_ANCHOR_INSET_Y, OVERLAY_H, OVERLAY_W, TICK_MS};
use crate::live;
use crate::pack;
use crate::session::{self, Target};
use crate::settings::{OverlayShowMode, Settings};
use crate::whip::{Frame, Phase, WhipThenNext};

pub const W: usize = OVERLAY_W;
pub const H: usize = OVERLAY_H;

pub struct Skin {
    pub id: String,
    pub idle: Vec<Frame>,
    pub crack: Vec<Frame>,
    pub pix: Vec<Vec<u32>>,
    pub rgba: Vec<Vec<u8>>,
    pub w: usize,
    pub h: usize,
}

pub fn overlay_pixel_size(scale_percent: u32) -> (usize, usize) {
    layout::overlay_pixel_size(scale_percent)
}

pub fn overlay_size_i32(scale_percent: u32) -> (i32, i32) {
    layout::overlay_size_i32(scale_percent)
}

pub fn parse_vk(name: &str) -> Option<i32> {
    match name.trim().to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(0x11),
        "alt" | "menu" => Some(0x12),
        "shift" => Some(0x10),
        "win" | "meta" | "super" => Some(0x5B),
        "caps" | "capslock" => Some(0x14),
        "space" => Some(0x20),
        "" | "none" | "off" => None,
        other => {
            if let Some(hex) = other.strip_prefix("0x") {
                i32::from_str_radix(hex, 16).ok()
            } else {
                other.parse().ok()
            }
        }
    }
}

pub fn overlay_key_visible(mode: OverlayShowMode, key_held: bool) -> bool {
    match mode {
        OverlayShowMode::Always => true,
        OverlayShowMode::WhileHeld => key_held,
        OverlayShowMode::HideWhileHeld => !key_held,
    }
}

/// Same rule as `dev`: surface present and not hide-now, then the hold-key mode.
pub fn overlay_is_shown(
    surface: Option<&str>,
    hide_now: bool,
    mode: OverlayShowMode,
    key_held: bool,
) -> bool {
    surface.is_some() && !hide_now && overlay_key_visible(mode, key_held)
}

pub fn load_skin_from(root: &Path, id: &str) -> Result<Skin, String> {
    load_skin_sized(root, id, W as u32, H as u32)
}

pub fn load_skin_sized(root: &Path, id: &str, w: u32, h: u32) -> Result<Skin, String> {
    if pack::model_is_3d(root, id) {
        match load_skin_gltf(root, id, w, h) {
            Ok(skin) => return Ok(skin),
            Err(_) if id != "default" => {
                return load_skin_sized(root, "default", w, h);
            }
            Err(e) => return Err(e),
        }
    }
    let (idle_p, act_p) =
        pack::frame_paths(root, id).or_else(|_| pack::frame_paths(root, "default"))?;
    let mut idle = Vec::new();
    let mut crack = Vec::new();
    let mut pix = Vec::new();
    let mut rgba = Vec::new();
    for path in idle_p {
        push_source_frames(&path, w, h, &mut idle, &mut pix, &mut rgba);
    }
    for path in act_p {
        push_source_frames(&path, w, h, &mut crack, &mut pix, &mut rgba);
    }
    if idle.is_empty() {
        return Err(format!("model '{id}' has no idle frames"));
    }
    if crack.len() < crate::layout::MIN_ACTION_FRAMES {
        return Err(format!("model '{id}' needs at least 2 action frames"));
    }
    Ok(Skin {
        id: id.to_string(),
        idle,
        crack,
        pix,
        rgba,
        w: w as usize,
        h: h as usize,
    })
}

#[inline(never)]
fn load_skin_gltf(root: &Path, id: &str, w: u32, h: u32) -> Result<Skin, String> {
    let src = pack::gltf_source(root, id).ok_or_else(|| format!("model '{id}' has no glTF"))?;
    let mesh = crate::gltf::load(&src)?;
    let m = pack::read_manifest(root, id).unwrap_or_else(|_| pack::default_manifest(id));
    let (idle_frames, action_frames) = crate::gltf::overlay_frames(
        &mesh,
        m.idle.first().map(|s| s.as_str()),
        m.action.first().map(|s| s.as_str()),
        w,
        h,
    )?;
    let mut idle = Vec::new();
    let mut crack = Vec::new();
    let mut pix = Vec::new();
    let mut rgba = Vec::new();
    let src_s = src.display().to_string();
    for (name, raw) in idle_frames {
        idle.push(Frame {
            id: name,
            path: src_s.clone(),
        });
        let mut raw = raw;
        paint_click_pad(&mut raw, w as i32, h as i32);
        pix.push(rgba_to_minifb(&raw));
        rgba.push(raw);
    }
    for (name, raw) in action_frames {
        crack.push(Frame {
            id: name,
            path: src_s.clone(),
        });
        let mut raw = raw;
        paint_click_pad(&mut raw, w as i32, h as i32);
        pix.push(rgba_to_minifb(&raw));
        rgba.push(raw);
    }
    Ok(Skin {
        id: id.to_string(),
        idle,
        crack,
        pix,
        rgba,
        w: w as usize,
        h: h as usize,
    })
}

fn push_source_frames(
    path: &Path,
    w: u32,
    h: u32,
    dest: &mut Vec<Frame>,
    pix: &mut Vec<Vec<u32>>,
    rgba: &mut Vec<Vec<u8>>,
) {
    let src = path.display().to_string();
    for (name, raw) in load_frames_rgba(path, w, h) {
        dest.push(Frame {
            id: name,
            path: src.clone(),
        });
        let mut raw = raw;
        paint_click_pad(&mut raw, w as i32, h as i32);
        pix.push(rgba_to_minifb(&raw));
        rgba.push(raw);
    }
}

/// GIF sources expand into every frame; SVG/3D fall back to a sibling PNG.
#[inline(never)]
pub fn load_frames_rgba(path: &Path, w: u32, h: u32) -> Vec<(String, Vec<u8>)> {
    match pack::media_kind_of(path) {
        "gif" => {
            if let Some(frames) = decode_gif_frames(path, w, h).filter(|f| !f.is_empty()) {
                return frames;
            }
        }
        "video" => match crate::video::decode_frames(path, w, h) {
            Ok(frames) if !frames.is_empty() => return frames,
            _ => return Vec::new(),
        },
        _ => {}
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    vec![(name, load_frame_rgba(path, w, h))]
}

#[inline(never)]
fn decode_gif_frames(path: &Path, w: u32, h: u32) -> Option<Vec<(String, Vec<u8>)>> {
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;
    let file = File::open(path).ok()?;
    let decoder = GifDecoder::new(BufReader::new(file)).ok()?;
    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
    let mut frames = Vec::new();
    for (i, item) in decoder.into_frames().enumerate() {
        if let Ok(frame) = item {
            let resized = image::imageops::resize(
                &frame.into_buffer(),
                w,
                h,
                image::imageops::FilterType::Triangle,
            );
            let mut raw = resized.into_raw();
            key_magenta(&mut raw);
            frames.push((format!("{stem}_{i:02}"), raw));
        }
    }
    Some(frames)
}

pub fn load_skin_from_roots(
    bundled: &Path,
    catalog: &Path,
    id: &str,
    w: u32,
    h: u32,
) -> Result<Skin, String> {
    let root = pack::resolve_pack_root(bundled, catalog, id);
    load_skin_sized(&root, id, w, h)
}

pub fn load_skin(id: &str) -> Result<Skin, String> {
    load_skin_from(&pack::default_pack_root(), id)
}

fn key_magenta(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let r = px[0] as i16;
        let g = px[1] as i16;
        let b = px[2] as i16;
        if r > 180 && b > 160 && g < 140 && (r - g) > 30 && (b - g) > 20 {
            px[3] = 0;
        }
    }
}

fn open_raster(path: &Path) -> Option<image::DynamicImage> {
    if let Ok(img) = image::open(path) {
        return Some(img);
    }
    let png = path.with_extension("png");
    if png != path {
        image::open(&png).ok()
    } else {
        None
    }
}

/// Keep PNG alpha. Transparent surround stays a=0 — never fill black.
/// SVG uses a sibling `.png` with the same stem. 3D is software-rasterized.
pub fn load_frame_rgba(path: &Path, w: u32, h: u32) -> Vec<u8> {
    if pack::media_kind_of(path) == "3d" {
        let px = crate::gltf::raster_file(path, w, h);
        if px.chunks(4).any(|p| p[3] > ALPHA_HIT) {
            return px;
        }
    }
    let Some(img) = open_raster(path) else {
        return vec![0; (w.max(1) as usize) * (h.max(1) as usize) * 4];
    };
    let resized =
        image::imageops::resize(&img.to_rgba8(), w, h, image::imageops::FilterType::Triangle);
    let mut raw = resized.into_raw();
    key_magenta(&mut raw);
    raw
}

/// Premultiply RGBA → BGRA for UpdateLayeredWindow.
pub fn premultiply_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; rgba.len()];
    for (src, dst) in rgba.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        let a = src[3] as u16;
        dst[0] = ((src[2] as u16 * a) / 255) as u8;
        dst[1] = ((src[1] as u16 * a) / 255) as u8;
        dst[2] = ((src[0] as u16 * a) / 255) as u8;
        dst[3] = src[3];
    }
    out
}

pub fn rgba_to_minifb(rgba: &[u8]) -> Vec<u32> {
    rgba.chunks_exact(4)
        .map(|p| {
            let (r, g, b, a) = (p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32);
            if a < ALPHA_HIT as u32 {
                0
            } else {
                (r << 16) | (g << 8) | b
            }
        })
        .collect()
}

fn in_disc(lx: i32, ly: i32, cx: i32, cy: i32, r: i32) -> bool {
    let dx = lx - cx;
    let dy = ly - cy;
    dx * dx + dy * dy <= r * r
}

/// Magenta-keyed sprites leave the cursor sitting in a hole. Stamp a 1-alpha
/// disc at the torso and at the cursor anchor so layered hit-testing sees them.
fn paint_click_pad(rgba: &mut [u8], w: i32, h: i32) {
    let r = w.min(h) / 5;
    stamp_disc(rgba, w, h, w / 2, h * 2 / 3, r);
    let ay = h - OVERLAY_ANCHOR_INSET_Y;
    if ay >= 0 {
        stamp_disc(rgba, w, h, w / 2, ay, r);
    }
}

fn stamp_disc(rgba: &mut [u8], w: i32, h: i32, cx: i32, cy: i32, r: i32) {
    for y in 0..h {
        for x in 0..w {
            if !in_disc(x, y, cx, cy, r) {
                continue;
            }
            let i = ((y as usize) * (w as usize) + (x as usize)) * 4 + 3;
            if i < rgba.len() && rgba[i] == 0 {
                rgba[i] = 1;
            }
        }
    }
}

/// Opaque pixel, or a disc over the torso / cursor so sparse sprites still receive clicks.
pub fn click_hits_sprite(rgba: &[u8], w: i32, h: i32, lx: i32, ly: i32) -> bool {
    if w <= 0 || h <= 0 || lx < 0 || ly < 0 || lx >= w || ly >= h {
        return false;
    }
    let i = ((ly as usize) * (w as usize) + (lx as usize)) * 4 + 3;
    if rgba.get(i).copied().unwrap_or(0) > ALPHA_HIT {
        return true;
    }
    let r = w.min(h) / 5;
    if in_disc(lx, ly, w / 2, h * 2 / 3, r) {
        return true;
    }
    let ay = h - OVERLAY_ANCHOR_INSET_Y;
    ay >= 0 && in_disc(lx, ly, w / 2, ay, r)
}

pub fn overlay_origin(cursor: (i32, i32), w: i32, h: i32) -> (i32, i32) {
    (
        cursor.0.saturating_sub(w / 2),
        cursor.1.saturating_sub(h - OVERLAY_ANCHOR_INSET_Y),
    )
}

pub fn current_rgba<'a>(
    machine: &WhipThenNext<RecordingInjector, OsAudio>,
    rgba: &'a [Vec<u8>],
) -> &'a [u8] {
    frame_slice(machine.phase, machine.index, machine.idle.len(), rgba)
}

pub fn frame_slice<T>(phase: Phase, index: usize, idle_n: usize, frames: &[T]) -> &T {
    match phase {
        Phase::Idle => &frames[index % idle_n],
        Phase::Crack => &frames[idle_n + index],
    }
}

fn demo_target() -> Target {
    Target {
        kind: crate::kind::Kind::Grok,
        session_id: Some("front-grok".into()),
        pid: Some(222),
        pane_id: Some("wG:p4".into()),
        herdr_name: Some("worker-1".into()),
    }
}

pub fn resolve_machine_target(demo: bool, force: Option<Target>) -> Option<Target> {
    if let Some(t) = force {
        return Some(t);
    }
    if demo {
        Some(demo_target())
    } else {
        live::resolve_live_target()
    }
}

pub fn log_line(path: &Option<std::path::PathBuf>, v: serde_json::Value) {
    use std::io::Write;
    let line = v.to_string();
    println!("{line}");
    if let Some(p) = path {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

pub fn dpi_aware() {
    #[cfg(windows)]
    unsafe {
        #[link(name = "user32")]
        extern "system" {
            fn SetProcessDPIAware() -> i32;
        }
        SetProcessDPIAware();
    }
}

pub struct LaunchOpts {
    pub demo: bool,
    pub auto_click: bool,
    pub quit_after: bool,
    pub log_path: Option<std::path::PathBuf>,
    pub timeout: Option<Duration>,
    pub dump_dir: Option<std::path::PathBuf>,
    pub force_target: Option<Target>,
    pub always_show: bool,
    pub settings: Settings,
    pub live: Option<Arc<Mutex<Settings>>>,
    pub quit: Option<Arc<AtomicBool>>,
}

pub trait OverlayHost {
    fn now(&self) -> Instant;
    fn pump(&mut self);
    fn quit_requested(&self) -> bool;
    fn live_settings(&self) -> Option<Settings>;
    fn cursor(&self) -> Option<(i32, i32)>;
    fn foreground(&self) -> Option<Foreground>;
    fn clicked_opaque(&mut self, rgba: &[u8], win_x: i32, win_y: i32) -> bool;
    fn set_visible(&mut self, vis: bool);
    fn set_pos(&mut self, x: i32, y: i32);
    fn blit_rgba(&mut self, rgba: &[u8]);
    fn escape_over_window(&self, win_x: i32, win_y: i32) -> bool;
    fn sleep_frame(&mut self);
    fn live_inject(&mut self, target: &Target, payload: &str) -> Result<(), String>;
    fn hold_key_down(&self, _name: &str) -> bool {
        false
    }
    fn set_pixel_size(&mut self, _w: i32, _h: i32) {}
}

pub struct OverlayEngine {
    pub machine: WhipThenNext<RecordingInjector, OsAudio>,
    pub rgba: Vec<Vec<u8>>,
    pub pix: Vec<Vec<u32>>,
    pub skin_id: String,
    pub settings: Settings,
    pub visible: bool,
    pub last_surface: Option<String>,
    pub last_tick: Instant,
    pub win_x: i32,
    pub win_y: i32,
    pub start: Instant,
    pub pack_root: std::path::PathBuf,
    pub catalog_root: std::path::PathBuf,
    pub pixel_w: usize,
    pub pixel_h: usize,
    pub click_fg: Option<Foreground>,
}

impl OverlayEngine {
    pub fn from_skin(
        skin: Skin,
        machine: WhipThenNext<RecordingInjector, OsAudio>,
        settings: Settings,
        now: Instant,
        pack_root: std::path::PathBuf,
    ) -> Result<Self, String> {
        let need = skin.w * skin.h * 4;
        if skin.rgba.iter().any(|b| b.len() != need) {
            return Err("frame decode failed".into());
        }
        Ok(Self {
            machine,
            pixel_w: skin.w,
            pixel_h: skin.h,
            rgba: skin.rgba,
            pix: skin.pix,
            skin_id: skin.id,
            settings,
            visible: true,
            last_surface: None,
            last_tick: now,
            win_x: 0,
            win_y: 0,
            start: now,
            pack_root,
            catalog_root: std::path::PathBuf::new(),
            click_fg: None,
        })
    }

    pub fn current_bytes(&self) -> &[u8] {
        current_rgba(&self.machine, &self.rgba)
    }

    fn apply_surface(&mut self, surface: Option<&str>) {
        let want = self.settings.model_for(surface).to_string();
        let root = pack::resolve_pack_root(&self.pack_root, &self.catalog_root, &want);
        self.machine.audio.whip = self.settings.resolved_sound_whip(
            surface,
            pack::sound_action_from_manifest(&root, &want).as_deref(),
        );
        self.machine.audio.next = self.settings.resolved_sound_next(
            surface,
            pack::sound_next_from_manifest(&root, &want).as_deref(),
        );
        if self.settings.phrases.contains_key(&want) {
            self.machine.payload = self.settings.phrase_for(&want);
        } else {
            self.machine.payload = pack::phrase_from_manifest(&root, &want);
        }
        if want == self.skin_id || self.machine.phase != Phase::Idle {
            return;
        }
        let (w, h) = overlay_pixel_size(self.settings.overlay_scale);
        let skin = match load_skin_from_roots(
            &self.pack_root,
            &self.catalog_root,
            &want,
            w as u32,
            h as u32,
        ) {
            Ok(skin) => skin,
            Err(_) => return,
        };
        self.apply_skin(skin);
    }

    fn apply_skin(&mut self, skin: Skin) {
        self.machine.idle = skin.idle;
        self.machine.crack = skin.crack;
        self.machine.index = 0;
        self.pixel_w = skin.w;
        self.pixel_h = skin.h;
        self.rgba = skin.rgba;
        self.pix = skin.pix;
        self.skin_id = skin.id;
    }

    fn rescale_if_needed(&mut self, host: &mut dyn OverlayHost) {
        let (w, h) = overlay_pixel_size(self.settings.overlay_scale);
        if w == self.pixel_w && h == self.pixel_h {
            return;
        }
        if self.machine.phase != Phase::Idle {
            return;
        }
        let skin = match load_skin_from_roots(
            &self.pack_root,
            &self.catalog_root,
            &self.skin_id,
            w as u32,
            h as u32,
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        self.apply_skin(skin);
        host.set_pixel_size(w as i32, h as i32);
    }
}

pub fn run_with_host(
    opts: LaunchOpts,
    mut engine: OverlayEngine,
    host: &mut dyn OverlayHost,
) -> Result<OverlayEngine, String> {
    log_line(
        &opts.log_path,
        serde_json::json!({
            "event": "surface",
            "shown": true,
            "alpha": true,
            "frame": engine.machine.frame_id(),
            "phase": "idle",
            "size": format!("{}x{}", engine.pixel_w, engine.pixel_h),
            "idle_frames": engine.machine.idle.iter().map(|f| &f.id).collect::<Vec<_>>(),
            "punch_frames": engine.machine.crack.iter().map(|f| &f.id).collect::<Vec<_>>(),
            "demo": opts.demo,
        }),
    );
    if opts.auto_click {
        let started = engine.machine.handle_click();
        log_line(
            &opts.log_path,
            serde_json::json!({"event":"click","started": started, "phase":"crack","frame": engine.machine.frame_id()}),
        );
    }
    let mut submitted = engine.machine.injector.calls.len();
    loop {
        host.pump();
        if opts
            .quit
            .as_ref()
            .is_some_and(|q| q.load(Ordering::Relaxed))
            || host.quit_requested()
        {
            break;
        }
        if let Some(s) = host.live_settings() {
            engine.settings = s;
        }
        engine.rescale_if_needed(host);
        let now = host.now();
        if let Some(t) = opts.timeout {
            if now.duration_since(engine.start) > t {
                break;
            }
        }
        let foreground = host.foreground();
        let surface = focus::overlay_surface(
            opts.always_show,
            foreground.as_ref(),
            engine.last_surface.as_deref(),
            &engine.settings,
        );
        engine.apply_surface(surface.as_deref());
        let held = host.hold_key_down(&engine.settings.overlay_key);
        let show = overlay_is_shown(
            surface.as_deref(),
            engine.settings.hide_now,
            engine.settings.overlay_mode,
            held,
        );
        if show != engine.visible {
            host.set_visible(show);
            engine.visible = show;
            log_line(
                &opts.log_path,
                serde_json::json!({
                    "event": "visibility",
                    "show": show,
                    "surface": surface,
                }),
            );
        }
        if surface != engine.last_surface {
            engine.last_surface = surface.clone();
        }
        if !engine.visible && engine.machine.phase != Phase::Crack {
            host.sleep_frame();
            continue;
        }
        if let Some(c) = host.cursor() {
            let (ow, oh) = overlay_size_i32(engine.settings.overlay_scale);
            let (x, y) = overlay_origin(c, ow, oh);
            engine.win_x = x;
            engine.win_y = y;
            host.set_pos(x, y);
        }
        if host.clicked_opaque(engine.current_bytes(), engine.win_x, engine.win_y) {
            engine.click_fg = host.foreground();
            let started = engine.machine.handle_click();
            log_line(
                &opts.log_path,
                serde_json::json!({
                    "event": "click",
                    "started": started,
                    "phase": format!("{:?}", engine.machine.phase).to_lowercase(),
                    "frame": engine.machine.frame_id(),
                }),
            );
        }
        if now.duration_since(engine.last_tick) >= Duration::from_millis(TICK_MS) {
            let was = engine.machine.phase;
            let was_id = engine.machine.frame_id().to_string();
            engine.machine.tick();
            if was == Phase::Crack {
                log_line(
                    &opts.log_path,
                    serde_json::json!({
                        "event": "punch",
                        "frame": was_id,
                        "next_frame": engine.machine.frame_id(),
                        "phase": format!("{:?}", engine.machine.phase).to_lowercase(),
                    }),
                );
            }
            if was == Phase::Crack && engine.machine.phase == Phase::Idle {
                let n = engine.machine.injector.calls.len();
                if n > submitted {
                    submitted = n;
                    let call = &engine.machine.injector.calls[n - 1];
                    log_line(
                        &opts.log_path,
                        serde_json::json!({
                            "event": "inject",
                            "session_id": call.target.session_id,
                            "pid": call.target.pid,
                            "pane_id": call.target.pane_id,
                            "payload": call.payload,
                            "submit": call.submit,
                        }),
                    );
                    if !opts.demo {
                        let live_fg = host.foreground();
                        let fg = engine.click_fg.as_ref().or(live_fg.as_ref());
                        let target = session::retarget_for_foreground(
                            call.target.clone(),
                            fg,
                            &engine.settings,
                        );
                        log_line(
                            &opts.log_path,
                            serde_json::json!({
                                "event": "retarget",
                                "pid": target.pid,
                                "pane_id": target.pane_id,
                            }),
                        );
                        if let Err(e) = host.live_inject(&target, &call.payload) {
                            log_line(
                                &opts.log_path,
                                serde_json::json!({"event":"inject-error","error": e}),
                            );
                        }
                    }
                }
                if opts.quit_after {
                    break;
                }
            }
            engine.last_tick = now;
        }
        host.blit_rgba(engine.current_bytes());
        if host.escape_over_window(engine.win_x, engine.win_y) && opts.quit.is_none() {
            break;
        }
        host.sleep_frame();
    }
    log_line(
        &opts.log_path,
        serde_json::json!({"event":"quit","ok": true}),
    );
    Ok(engine)
}

pub fn build_machine(
    skin: &Skin,
    demo: bool,
    force: Option<Target>,
) -> WhipThenNext<RecordingInjector, OsAudio> {
    WhipThenNext::new(
        skin.idle.clone(),
        skin.crack.clone(),
        RecordingInjector::default(),
        OsAudio::default(),
        Box::new(move || resolve_machine_target(demo, force.clone())),
    )
}

pub fn punch_frame(crack: &[Frame]) -> &Frame {
    crack.get(2).unwrap_or(&crack[crack.len() - 1])
}

#[inline(never)]
pub fn run_overlay(opts: LaunchOpts) -> Result<(), String> {
    let root = pack::default_pack_root();
    let catalog = pack::catalog_dir(&crate::settings::home_dir());
    let (pw, ph) = overlay_pixel_size(opts.settings.overlay_scale);
    let skin = load_skin_from_roots(&root, &catalog, &opts.settings.model, pw as u32, ph as u32)
        .or_else(|_| load_skin_sized(&root, "default", pw as u32, ph as u32))?;
    if let Some(dir) = &opts.dump_dir {
        let _ = std::fs::create_dir_all(dir);
        let idle = &skin.idle[0];
        let punch = punch_frame(&skin.crack);
        let _ = std::fs::copy(&idle.path, dir.join("overlay-idle.png"));
        let _ = std::fs::copy(&punch.path, dir.join("overlay-punch.png"));
    }
    let machine = build_machine(&skin, opts.demo, opts.force_target.clone());
    let mut engine =
        OverlayEngine::from_skin(skin, machine, opts.settings.clone(), Instant::now(), root)?;
    engine.catalog_root = catalog;
    dpi_aware();
    #[cfg(windows)]
    {
        let mut host = crate::overlay_win::WinHost::open(&opts)?;
        run_with_host(opts, engine, &mut host)?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let mut host = crate::overlay_unix::MiniFbHost::open(&opts)?;
        run_with_host(opts, engine, &mut host)?;
        Ok(())
    }
}
