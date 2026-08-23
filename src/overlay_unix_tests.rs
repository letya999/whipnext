//! minifb overlay host (non-Windows).

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

use crate::focus::Foreground;
use crate::overlay::{log_line, rgba_to_minifb, LaunchOpts, OverlayHost, H, W};
use crate::session::Target;
use crate::settings::Settings;

pub struct MiniFbHost {
    win: minifb::Window,
    mouse_was_down: bool,
    quit: Option<Arc<AtomicBool>>,
    live: Option<Arc<Mutex<Settings>>>,
    foreground_cache: Mutex<Option<(Instant, Option<Foreground>)>>,
}

impl MiniFbHost {
    pub fn open(opts: &LaunchOpts) -> Result<Self, String> {
        use minifb::{Scale, Window, WindowOptions};
        let win = Window::new(
            "whipnext",
            W,
            H,
            WindowOptions {
                title: false,
                borderless: true,
                resize: false,
                transparency: true,
                topmost: true,
                none: true,
                scale: Scale::X1,
                ..WindowOptions::default()
            },
        )
        .map_err(|e| {
            log_line(
                &opts.log_path,
                serde_json::json!({"event":"launch-unavailable","error": e.to_string()}),
            );
            format!("overlay: {e}")
        })?;
        Ok(Self {
            win,
            mouse_was_down: false,
            quit: opts.quit.clone(),
            live: opts.live.clone(),
            foreground_cache: Mutex::new(None),
        })
    }
}

impl OverlayHost for MiniFbHost {
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn pump(&mut self) {
        self.win.set_target_fps(30);
    }
    fn quit_requested(&self) -> bool {
        use minifb::Key;
        self.quit
            .as_ref()
            .is_some_and(|q| q.load(Ordering::Relaxed))
            || !self.win.is_open()
            || self.win.is_key_down(Key::Escape)
    }
    fn live_settings(&self) -> Option<Settings> {
        self.live
            .as_ref()
            .and_then(|l| l.lock().ok().map(|g| g.clone()))
    }
    fn cursor(&self) -> Option<(i32, i32)> {
        crate::os_probe::cursor_pos()
    }
    fn foreground(&self) -> Option<Foreground> {
        if let Ok(cache) = self.foreground_cache.lock() {
            if let Some((at, value)) = &*cache {
                if at.elapsed() < std::time::Duration::from_millis(100) {
                    return value.clone();
                }
            }
        }
        let value = crate::os_probe::foreground_snapshot();
        if let Ok(mut cache) = self.foreground_cache.lock() {
            *cache = Some((Instant::now(), value.clone()));
        }
        value
    }
    fn clicked_opaque(&mut self, _rgba: &[u8], _win_x: i32, _win_y: i32) -> bool {
        use minifb::MouseButton;
        let down = self.win.get_mouse_down(MouseButton::Left);
        let edge = down && !self.mouse_was_down;
        self.mouse_was_down = down;
        edge
    }
    fn set_visible(&mut self, _vis: bool) {}
    fn set_pos(&mut self, x: i32, y: i32) {
        self.win.set_position(x as isize, y as isize);
    }
    fn blit_rgba(&mut self, rgba: &[u8]) {
        let pix = rgba_to_minifb(rgba);
        let _ = self.win.update_with_buffer(&pix, W, H);
    }
    fn escape_over_window(&self, _win_x: i32, _win_y: i32) -> bool {
        false
    }
    fn sleep_frame(&mut self) {}
    fn live_inject(&mut self, target: &Target, payload: &str) -> Result<(), String> {
        crate::inject_os::live_inject_with(target, payload)
    }
}
