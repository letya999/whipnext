//! Per-pixel alpha overlay via UpdateLayeredWindow. minifb cannot do this on
//! Windows (no WS_EX_LAYERED blit), so the black rectangle was a workaround.

#![cfg(windows)]

use std::mem::{size_of, zeroed};
use std::ptr;

const WS_POPUP: u32 = 0x80000000;
const WS_EX_LAYERED: u32 = 0x00080000;
const WS_EX_TOPMOST: u32 = 0x00000008;
const WS_EX_TOOLWINDOW: u32 = 0x00000080;
const WS_EX_NOACTIVATE: u32 = 0x08000000;
const HWND_TOPMOST: isize = -1;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_NOSIZE: u32 = 0x0001;
const SW_SHOWNOACTIVATE: i32 = 4;
const SW_HIDE: i32 = 0;
const ULW_ALPHA: u32 = 0x00000002;
const AC_SRC_OVER: u8 = 0;
const AC_SRC_ALPHA: u8 = 1;
const DIB_RGB_COLORS: u32 = 0;
const BI_RGB: u32 = 0;
const PM_REMOVE: u32 = 0x0001;
const VK_LBUTTON: i32 = 0x01;
const VK_ESCAPE: i32 = 0x1B;

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
struct Size {
    cx: i32,
    cy: i32,
}
#[repr(C)]
struct BlendFunction {
    blend_op: u8,
    blend_flags: u8,
    source_constant_alpha: u8,
    alpha_format: u8,
}
#[repr(C)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels: i32,
    bi_y_pels: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}
#[repr(C)]
struct BitmapInfo {
    hdr: BitmapInfoHeader,
    colors: [u32; 1],
}
#[repr(C)]
struct Msg {
    hwnd: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: Point,
}

#[link(name = "user32")]
extern "system" {
    fn CreateWindowExW(
        ex: u32,
        cls: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: isize,
        menu: isize,
        inst: isize,
        param: *mut core::ffi::c_void,
    ) -> isize;
    fn RegisterClassW(c: *const WndClass) -> u16;
    fn DefWindowProcW(h: isize, m: u32, w: usize, l: isize) -> isize;
    fn ShowWindow(h: isize, cmd: i32) -> i32;
    fn SetWindowPos(h: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    fn UpdateLayeredWindow(
        h: isize,
        hdc_dst: isize,
        dst: *const Point,
        size: *const Size,
        hdc_src: isize,
        src: *const Point,
        key: u32,
        blend: *const BlendFunction,
        flags: u32,
    ) -> i32;
    fn GetDC(h: isize) -> isize;
    fn ReleaseDC(h: isize, dc: isize) -> i32;
    fn PeekMessageW(m: *mut Msg, h: isize, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(m: *const Msg) -> i32;
    fn DispatchMessageW(m: *const Msg) -> isize;
    fn DestroyWindow(h: isize) -> i32;
    fn GetAsyncKeyState(v: i32) -> i16;
    fn GetCursorPos(p: *mut Point) -> i32;
    fn WindowFromPoint(p: Point) -> isize;
    fn GetAncestor(h: isize, flags: u32) -> isize;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: isize) -> isize;
    fn CreateDIBSection(
        hdc: isize,
        bmi: *const BitmapInfo,
        usage: u32,
        bits: *mut *mut u8,
        section: isize,
        offset: u32,
    ) -> isize;
    fn SelectObject(hdc: isize, obj: isize) -> isize;
    fn DeleteDC(hdc: isize) -> i32;
    fn DeleteObject(obj: isize) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(n: *const u16) -> isize;
}

#[repr(C)]
struct WndClass {
    style: u32,
    wnd_proc: Option<unsafe extern "system" fn(isize, u32, usize, isize) -> isize>,
    cb_cls: i32,
    cb_wnd: i32,
    inst: isize,
    icon: isize,
    cursor: isize,
    brush: isize,
    menu: *const u16,
    class_name: *const u16,
}

unsafe extern "system" fn wnd_proc(h: isize, m: u32, w: usize, l: isize) -> isize {
    DefWindowProcW(h, m, w, l)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub use crate::overlay::premultiply_bgra;

pub struct Layered {
    hwnd: isize,
    hdc_mem: isize,
    hbmp: isize,
    bits: *mut u8,
    w: i32,
    h: i32,
    prev_left: bool,
}

impl Layered {
    pub fn new(w: i32, h: i32) -> Result<Self, String> {
        unsafe {
            let inst = GetModuleHandleW(ptr::null());
            let cls = wide("whipnext_layered");
            let wc = WndClass {
                style: 0,
                wnd_proc: Some(wnd_proc),
                cb_cls: 0,
                cb_wnd: 0,
                inst,
                icon: 0,
                cursor: 0,
                brush: 0,
                menu: ptr::null(),
                class_name: cls.as_ptr(),
            };
            RegisterClassW(&wc);
            let title = wide("whipnext");
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                cls.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                0,
                0,
                w,
                h,
                0,
                0,
                inst,
                ptr::null_mut(),
            );
            if hwnd == 0 {
                return Err("CreateWindowExW failed".into());
            }
            let hdc_screen = GetDC(0);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            ReleaseDC(0, hdc_screen);
            let mut bmi: BitmapInfo = zeroed();
            bmi.hdr.bi_size = size_of::<BitmapInfoHeader>() as u32;
            bmi.hdr.bi_width = w;
            bmi.hdr.bi_height = -h; // top-down
            bmi.hdr.bi_planes = 1;
            bmi.hdr.bi_bit_count = 32;
            bmi.hdr.bi_compression = BI_RGB;
            let mut bits: *mut u8 = ptr::null_mut();
            let hbmp = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, 0, 0);
            if hbmp == 0 || bits.is_null() {
                return Err("CreateDIBSection failed".into());
            }
            SelectObject(hdc_mem, hbmp);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            Ok(Self {
                hwnd,
                hdc_mem,
                hbmp,
                bits,
                w,
                h,
                prev_left: false,
            })
        }
    }

    pub fn set_pos(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    pub fn blit_rgba(&self, rgba: &[u8]) {
        let bgra = premultiply_bgra(rgba);
        let n = (self.w * self.h * 4) as usize;
        if bgra.len() < n {
            return;
        }
        unsafe {
            ptr::copy_nonoverlapping(bgra.as_ptr(), self.bits, n);
            let src = Point { x: 0, y: 0 };
            let size = Size {
                cx: self.w,
                cy: self.h,
            };
            let blend = BlendFunction {
                blend_op: AC_SRC_OVER,
                blend_flags: 0,
                source_constant_alpha: 255,
                alpha_format: AC_SRC_ALPHA,
            };
            UpdateLayeredWindow(
                self.hwnd,
                0,
                ptr::null(),
                &size,
                self.hdc_mem,
                &src,
                0,
                &blend,
                ULW_ALPHA,
            );
        }
    }

    pub fn pump(&self) {
        unsafe {
            let mut msg: Msg = zeroed();
            while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// Rising edge of left click on an opaque pixel of THIS overlay, not the settings window.
    pub fn clicked_opaque(&mut self, rgba: &[u8], win_x: i32, win_y: i32) -> bool {
        let down = unsafe { GetAsyncKeyState(VK_LBUTTON) as u16 & 0x8000 != 0 };
        let edge = down && !self.prev_left;
        self.prev_left = down;
        if !edge {
            return false;
        }
        let mut pt = Point { x: 0, y: 0 };
        unsafe {
            GetCursorPos(&mut pt);
        }
        let hit = unsafe { WindowFromPoint(pt) };
        let root = unsafe { GetAncestor(hit, 2) };
        if hit != self.hwnd && root != self.hwnd {
            return false;
        }
        let lx = pt.x - win_x;
        let ly = pt.y - win_y;
        crate::overlay::click_hits_sprite(rgba, self.w, self.h, lx, ly)
    }

    pub fn set_visible(&self, vis: bool) {
        unsafe {
            ShowWindow(self.hwnd, if vis { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }

    pub fn escape_over_window(&self, win_x: i32, win_y: i32) -> bool {
        let down = unsafe { GetAsyncKeyState(VK_ESCAPE) as u16 & 0x8000 != 0 };
        if !down {
            return false;
        }
        let mut pt = Point { x: 0, y: 0 };
        unsafe {
            GetCursorPos(&mut pt);
        }
        pt.x >= win_x && pt.y >= win_y && pt.x < win_x + self.w && pt.y < win_y + self.h
    }
}

impl Drop for Layered {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
            DeleteObject(self.hbmp);
            DeleteDC(self.hdc_mem);
        }
    }
}

pub struct WinHost {
    layered: Layered,
    quit: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    live: Option<std::sync::Arc<std::sync::Mutex<crate::settings::Settings>>>,
    dev: bool,
    foreground_cache:
        std::sync::Mutex<Option<(std::time::Instant, Option<crate::focus::Foreground>)>>,
}

impl WinHost {
    pub fn open(opts: &crate::overlay::LaunchOpts) -> Result<Self, String> {
        let (w, h) = crate::overlay::overlay_size_i32(opts.settings.overlay_scale);
        Ok(Self {
            layered: Layered::new(w, h)?,
            quit: opts.quit.clone(),
            live: opts.live.clone(),
            dev: opts.settings.dev_mode,
            foreground_cache: std::sync::Mutex::new(None),
        })
    }
}

impl crate::overlay::OverlayHost for WinHost {
    fn now(&self) -> std::time::Instant {
        std::time::Instant::now()
    }
    fn pump(&mut self) {
        self.layered.pump();
    }
    fn quit_requested(&self) -> bool {
        self.quit
            .as_ref()
            .is_some_and(|q| q.load(std::sync::atomic::Ordering::Relaxed))
    }
    fn live_settings(&self) -> Option<crate::settings::Settings> {
        self.live
            .as_ref()
            .and_then(|l| l.lock().ok().map(|g| g.clone()))
    }
    fn cursor(&self) -> Option<(i32, i32)> {
        crate::os_probe::cursor_pos()
    }
    fn foreground(&self) -> Option<crate::focus::Foreground> {
        if let Ok(cache) = self.foreground_cache.lock() {
            if let Some((at, value)) = &*cache {
                if at.elapsed() < std::time::Duration::from_millis(100) {
                    return value.clone();
                }
            }
        }
        let value = crate::os_probe::foreground_snapshot();
        if let Ok(mut cache) = self.foreground_cache.lock() {
            *cache = Some((std::time::Instant::now(), value.clone()));
        }
        value
    }
    fn clicked_opaque(&mut self, rgba: &[u8], win_x: i32, win_y: i32) -> bool {
        self.layered.clicked_opaque(rgba, win_x, win_y)
    }
    fn set_visible(&mut self, vis: bool) {
        self.layered.set_visible(vis);
    }
    fn set_pos(&mut self, x: i32, y: i32) {
        self.layered.set_pos(x, y);
    }
    fn blit_rgba(&mut self, rgba: &[u8]) {
        self.layered.blit_rgba(rgba);
    }
    fn escape_over_window(&self, win_x: i32, win_y: i32) -> bool {
        self.layered.escape_over_window(win_x, win_y)
    }
    fn sleep_frame(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    fn live_inject(
        &mut self,
        target: &crate::session::Target,
        payload: &str,
    ) -> Result<(), String> {
        let target = target.clone();
        let payload = payload.to_string();
        let dev = self.dev || self.live_settings().is_some_and(|s| s.dev_mode);
        let sink = if dev {
            Some(crate::inject::dev_sink(&crate::settings::home_dir()))
        } else {
            None
        };
        std::thread::spawn(move || {
            let _ = crate::inject_os::live_inject_with_dev(&target, &payload, sink);
        });
        Ok(())
    }
    fn hold_key_down(&self, name: &str) -> bool {
        let Some(vk) = crate::overlay::parse_vk(name) else {
            return false;
        };
        unsafe { GetAsyncKeyState(vk) as u16 & 0x8000 != 0 }
    }
    fn set_pixel_size(&mut self, w: i32, h: i32) {
        if self.layered.w == w && self.layered.h == h {
            return;
        }
        if let Ok(n) = Layered::new(w, h) {
            self.layered = n;
        }
    }
}
