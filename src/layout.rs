//! Named overlay/settings sizes. Overlay, snapshot, and tests share these.

use serde_json::json;

pub const OVERLAY_W: usize = 280;
pub const OVERLAY_H: usize = 380;
pub const TICK_MS: u64 = 140;
pub const SCALE_MIN: u32 = 25;
pub const SCALE_MAX: u32 = 400;
pub const CONTROL_SCALE_MIN: u32 = 50;
pub const CONTROL_SCALE_MAX: u32 = 200;
pub const DEFAULT_SCALE: u32 = 100;
pub const MIN_OVERLAY_PX: usize = 32;
/// Overlay bottom sits this many CSS-pixels above the cursor hotspot.
pub const OVERLAY_ANCHOR_INSET_Y: i32 = 36;
pub const MAX_VIDEO_FRAMES: usize = 24;
pub const MIN_ACTION_FRAMES: usize = 2;
pub const ALPHA_HIT: u8 = 24;

pub fn overlay_pixel_size(scale_percent: u32) -> (usize, usize) {
    let s = scale_percent.clamp(SCALE_MIN, SCALE_MAX) as usize;
    let w = (OVERLAY_W * s / 100).max(MIN_OVERLAY_PX);
    let h = (OVERLAY_H * s / 100).max(MIN_OVERLAY_PX);
    (w, h)
}

pub fn overlay_size_i32(scale_percent: u32) -> (i32, i32) {
    let (w, h) = overlay_pixel_size(scale_percent);
    (w as i32, h as i32)
}

pub fn clamp_overlay_scale(v: u32) -> u32 {
    v.clamp(SCALE_MIN, SCALE_MAX)
}

pub fn clamp_control_scale(v: u32) -> u32 {
    v.clamp(CONTROL_SCALE_MIN, CONTROL_SCALE_MAX)
}

pub fn snapshot() -> serde_json::Value {
    json!({
        "overlay_w": OVERLAY_W,
        "overlay_h": OVERLAY_H,
        "tick_ms": TICK_MS,
        "scale_min": SCALE_MIN,
        "scale_max": SCALE_MAX,
        "control_scale_min": CONTROL_SCALE_MIN,
        "control_scale_max": CONTROL_SCALE_MAX,
        "min_px": MIN_OVERLAY_PX,
    })
}
