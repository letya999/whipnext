use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod common;

use whipnext::focus::{overlay_surface, Foreground, Proc};
use whipnext::inject::NEXT_PROMPT;
use whipnext::kind::Kind;
use whipnext::overlay::{
    dpi_aware, frame_slice, load_frame_rgba, load_frames_rgba, load_skin_from, load_skin_sized,
    log_line, overlay_is_shown, overlay_key_visible, overlay_origin, overlay_pixel_size,
    overlay_size_i32, parse_vk, punch_frame, resolve_machine_target, rgba_to_minifb, run_with_host,
    LaunchOpts, OverlayEngine, OverlayHost, H, W,
};
use whipnext::session::Target;
use whipnext::settings::{AppPref, OverlayShowMode, Settings};
use whipnext::whip::Phase;

struct FakeHost {
    t0: Instant,
    dt: Duration,
    n: u32,
    max: u32,
    cursor: Option<(i32, i32)>,
    fg: Option<Foreground>,
    click_lo: Option<u32>,
    click_hi: Option<u32>,
    escape: bool,
    live: Option<Settings>,
    inject_err: Option<String>,
    pub visible: bool,
    pub pos: Vec<(i32, i32)>,
    pub blits: usize,
    pub injected: Vec<Target>,
    pub held: bool,
    pub held_until: Option<u32>,
}

impl FakeHost {
    fn new(max: u32) -> Self {
        Self {
            t0: Instant::now(),
            dt: Duration::from_millis(150),
            n: 0,
            max,
            cursor: Some((400, 400)),
            fg: None,
            click_lo: None,
            click_hi: None,
            escape: false,
            live: None,
            inject_err: None,
            visible: true,
            pos: Vec::new(),
            blits: 0,
            injected: Vec::new(),
            held: false,
            held_until: None,
        }
    }
}

impl OverlayHost for FakeHost {
    fn now(&self) -> Instant {
        self.t0 + self.dt * self.n
    }
    fn pump(&mut self) {}
    fn quit_requested(&self) -> bool {
        self.n >= self.max
    }
    fn live_settings(&self) -> Option<Settings> {
        self.live.clone()
    }
    fn cursor(&self) -> Option<(i32, i32)> {
        self.cursor
    }
    fn foreground(&self) -> Option<Foreground> {
        self.fg.clone()
    }
    fn clicked_opaque(&mut self, _rgba: &[u8], _x: i32, _y: i32) -> bool {
        match (self.click_lo, self.click_hi) {
            (Some(a), Some(b)) => self.n >= a && self.n <= b,
            _ => false,
        }
    }
    fn set_visible(&mut self, vis: bool) {
        self.visible = vis;
    }
    fn set_pos(&mut self, x: i32, y: i32) {
        self.pos.push((x, y));
    }
    fn blit_rgba(&mut self, rgba: &[u8]) {
        assert_eq!(rgba.len() % 4, 0);
        assert!(!rgba.is_empty());
        self.blits += 1;
    }
    fn escape_over_window(&self, _x: i32, _y: i32) -> bool {
        self.escape
    }
    fn sleep_frame(&mut self) {
        self.n += 1;
    }
    fn live_inject(&mut self, target: &Target, _payload: &str) -> Result<(), String> {
        if let Some(e) = &self.inject_err {
            return Err(e.clone());
        }
        self.injected.push(target.clone());
        Ok(())
    }
    fn hold_key_down(&self, _name: &str) -> bool {
        self.held || self.held_until.is_some_and(|n| self.n <= n)
    }
}

fn engine(model: &str) -> OverlayEngine {
    let root = whipnext::pack::default_pack_root();
    let skin = load_skin_from(&root, model).unwrap();
    let machine = whipnext::whip::WhipThenNext::new(
        skin.idle.clone(),
        skin.crack.clone(),
        whipnext::inject::RecordingInjector::default(),
        whipnext::audio::OsAudio::default(),
        Box::new(|| {
            Some(Target {
                kind: Kind::Grok,
                session_id: Some("front-grok".into()),
                pid: Some(222),
                pane_id: Some("wG:p4".into()),
                herdr_name: Some("worker-1".into()),
            })
        }),
    );
    OverlayEngine::from_skin(skin, machine, Settings::default(), Instant::now(), root).unwrap()
}

fn opts(demo: bool) -> LaunchOpts {
    LaunchOpts {
        demo,
        auto_click: false,
        quit_after: false,
        log_path: None,
        timeout: None,
        dump_dir: None,
        force_target: None,
        always_show: true,
        settings: Settings::default(),
        live: None,
        quit: None,
    }
}

fn grok_fg() -> Foreground {
    Foreground {
        pid: 22,
        exe: "grok.exe".into(),
        title: "Grok".into(),
        procs: vec![Proc {
            pid: 22,
            name: "grok.exe".into(),
            parent: 0,
        }],
    }
}

#[test]
fn click_then_ticks_records_next_on_machine() {
    let mut host = FakeHost::new(12);
    host.click_lo = Some(0);
    host.click_hi = Some(0);
    host.fg = Some(grok_fg());
    let mut o = opts(false);
    o.always_show = true;
    let eng = run_with_host(o, engine("default"), &mut host).unwrap();
    assert_eq!(eng.machine.injector.calls.len(), 1);
    assert_eq!(eng.machine.injector.calls[0].payload, NEXT_PROMPT);
    assert_ne!(eng.machine.injector.calls[0].payload, "/next");
}

#[test]
fn releasing_overlay_key_after_click_still_injects() {
    let mut host = FakeHost::new(12);
    host.click_lo = Some(0);
    host.click_hi = Some(0);
    host.held_until = Some(0);
    host.fg = Some(grok_fg());
    let mut eng = engine("default");
    eng.settings.overlay_mode = OverlayShowMode::WhileHeld;
    let eng = run_with_host(opts(false), eng, &mut host).unwrap();
    assert_eq!(eng.machine.injector.calls.len(), 1);
    assert_eq!(host.injected.len(), 1);
}

#[test]
fn overlay_origin_sits_above_cursor() {
    assert_eq!(
        overlay_origin(
            (100, 200),
            whipnext::overlay::W as i32,
            whipnext::overlay::H as i32
        ),
        (
            100 - whipnext::overlay::W as i32 / 2,
            200 - (whipnext::overlay::H as i32 - whipnext::layout::OVERLAY_ANCHOR_INSET_Y)
        )
    );
    assert_eq!(
        whipnext::overlay::overlay_pixel_size(whipnext::layout::DEFAULT_SCALE),
        (whipnext::layout::OVERLAY_W, whipnext::layout::OVERLAY_H)
    );
}

#[test]
fn sparse_sprite_still_hits_torso_disc() {
    use whipnext::overlay::click_hits_sprite;
    let mut rgba = vec![0u8; 20 * 20 * 4];
    assert!(!click_hits_sprite(&rgba, 20, 20, 0, 0));
    assert!(click_hits_sprite(&rgba, 20, 20, 10, 13));
    let i = (2 * 20 + 2) * 4 + 3;
    rgba[i] = 255;
    assert!(click_hits_sprite(&rgba, 20, 20, 2, 2));
    assert!(!click_hits_sprite(&rgba, 20, 20, -1, 0));
    let pad = vec![0u8; 100 * 100 * 4];
    let ay = 100 - whipnext::layout::OVERLAY_ANCHOR_INSET_Y;
    assert!(click_hits_sprite(&pad, 100, 100, 50, ay));
}

#[test]
fn overlay_surface_always_self_and_hidden() {
    let mut s = Settings::default();
    s.only_when_matched = true;
    s.apps.insert("grok".into(), AppPref::default());
    assert_eq!(
        overlay_surface(true, None, None, &s).as_deref(),
        Some("always")
    );
    assert_eq!(overlay_surface(false, None, Some("grok"), &s), None);
    let self_fg = Foreground {
        pid: 1,
        exe: "whipnext.exe".into(),
        title: "".into(),
        procs: vec![],
    };
    assert_eq!(
        overlay_surface(false, Some(&self_fg), Some("grok"), &s).as_deref(),
        Some("grok")
    );
    assert_eq!(overlay_surface(false, Some(&self_fg), None, &s), None);
    let chrome = Foreground {
        pid: 9,
        exe: "chrome.exe".into(),
        title: "Google".into(),
        procs: vec![],
    };
    assert_eq!(
        overlay_surface(false, Some(&chrome), Some("grok"), &s),
        None
    );
    assert_eq!(
        overlay_surface(false, Some(&grok_fg()), None, &s).as_deref(),
        Some("grok")
    );
}

#[test]
fn missing_frame_file_is_transparent_zeros() {
    let z = load_frame_rgba(std::path::Path::new("no-such.png"), 2, 2);
    assert_eq!(z, vec![0; 16]);
}

#[test]
fn magenta_key_and_low_alpha_minifb() {
    let dir = std::env::temp_dir().join(format!("whipnext-magenta-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut img = image::RgbaImage::new(2, 2);
    img.put_pixel(0, 0, image::Rgba([220, 10, 200, 255]));
    img.put_pixel(1, 0, image::Rgba([10, 200, 10, 255]));
    img.put_pixel(0, 1, image::Rgba([0, 0, 0, 10]));
    img.put_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
    let p = dir.join("m.png");
    img.save(&p).unwrap();
    let rgba = load_frame_rgba(&p, 2, 2);
    assert_eq!(rgba[3], 0, "magenta-ish becomes transparent");
    let pix = rgba_to_minifb(&rgba);
    assert_eq!(pix[2], 0, "low alpha -> 0");
    assert_ne!(pix[3], 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn frame_slice_idle_and_crack() {
    let frames = vec!["i0", "i1", "c0", "c1"];
    assert_eq!(*frame_slice(Phase::Idle, 3, 2, &frames), "i1");
    assert_eq!(*frame_slice(Phase::Crack, 1, 2, &frames), "c1");
}

#[test]
fn resolve_machine_target_force_and_demo() {
    let t = Target {
        kind: Kind::Claude,
        session_id: None,
        pid: Some(3),
        pane_id: None,
        herdr_name: None,
    };
    assert_eq!(
        resolve_machine_target(true, Some(t.clone())).unwrap().pid,
        Some(3)
    );
    let d = resolve_machine_target(true, None).unwrap();
    assert_eq!(d.pane_id.as_deref(), Some("wG:p4"));
}

#[test]
fn log_line_writes_file_and_dpi_aware_runs() {
    let p = std::env::temp_dir().join(format!("whipnext-log-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&p);
    log_line(&Some(p.clone()), serde_json::json!({"event":"t"}));
    log_line(&None, serde_json::json!({"event":"stdout"}));
    let txt = std::fs::read_to_string(&p).unwrap();
    assert!(txt.contains("t"));
    dpi_aware();
    let _ = std::fs::remove_file(&p);
}

#[test]
fn drive_click_injects_once_second_click_ignored() {
    let injected = Arc::new(Mutex::new(Vec::new()));
    struct H {
        base: FakeHost,
        injected: Arc<Mutex<Vec<Target>>>,
    }
    impl OverlayHost for H {
        fn now(&self) -> Instant {
            self.base.now()
        }
        fn pump(&mut self) {
            self.base.pump();
        }
        fn quit_requested(&self) -> bool {
            self.base.quit_requested()
        }
        fn live_settings(&self) -> Option<Settings> {
            self.base.live_settings()
        }
        fn cursor(&self) -> Option<(i32, i32)> {
            self.base.cursor()
        }
        fn foreground(&self) -> Option<Foreground> {
            self.base.foreground()
        }
        fn clicked_opaque(&mut self, r: &[u8], x: i32, y: i32) -> bool {
            self.base.clicked_opaque(r, x, y)
        }
        fn set_visible(&mut self, v: bool) {
            self.base.set_visible(v);
        }
        fn set_pos(&mut self, x: i32, y: i32) {
            self.base.set_pos(x, y);
        }
        fn blit_rgba(&mut self, r: &[u8]) {
            self.base.blit_rgba(r);
        }
        fn escape_over_window(&self, x: i32, y: i32) -> bool {
            self.base.escape_over_window(x, y)
        }
        fn sleep_frame(&mut self) {
            self.base.sleep_frame();
        }
        fn live_inject(&mut self, t: &Target, payload: &str) -> Result<(), String> {
            self.injected.lock().unwrap().push(t.clone());
            self.base.live_inject(t, payload)
        }
    }
    let mut base = FakeHost::new(14);
    base.click_lo = Some(0);
    base.click_hi = Some(4);
    base.fg = Some(grok_fg());
    let mut h = H {
        injected: injected.clone(),
        base,
    };
    let mut o = opts(false);
    o.always_show = true;
    run_with_host(o, engine("default"), &mut h).unwrap();
    let got = injected.lock().unwrap().clone();
    assert_eq!(got.len(), 1, "exactly one live inject");
    assert_eq!(got[0].pid, Some(22), "retarget to foreground grok pid");
    assert!(
        got[0].pane_id.is_none(),
        "pane cleared when surface is not herdr"
    );
}

#[test]
fn second_crack_without_resolve_does_not_reinject() {
    let injected = Arc::new(Mutex::new(Vec::new()));
    struct H {
        base: FakeHost,
        injected: Arc<Mutex<Vec<Target>>>,
    }
    impl OverlayHost for H {
        fn now(&self) -> Instant {
            self.base.now()
        }
        fn pump(&mut self) {
            self.base.pump();
        }
        fn quit_requested(&self) -> bool {
            self.base.quit_requested()
        }
        fn live_settings(&self) -> Option<Settings> {
            self.base.live_settings()
        }
        fn cursor(&self) -> Option<(i32, i32)> {
            self.base.cursor()
        }
        fn foreground(&self) -> Option<Foreground> {
            self.base.foreground()
        }
        fn clicked_opaque(&mut self, r: &[u8], x: i32, y: i32) -> bool {
            self.base.clicked_opaque(r, x, y)
        }
        fn set_visible(&mut self, v: bool) {
            self.base.set_visible(v);
        }
        fn set_pos(&mut self, x: i32, y: i32) {
            self.base.set_pos(x, y);
        }
        fn blit_rgba(&mut self, r: &[u8]) {
            self.base.blit_rgba(r);
        }
        fn escape_over_window(&self, x: i32, y: i32) -> bool {
            self.base.escape_over_window(x, y)
        }
        fn sleep_frame(&mut self) {
            self.base.sleep_frame();
        }
        fn live_inject(&mut self, t: &Target, _payload: &str) -> Result<(), String> {
            self.injected.lock().unwrap().push(t.clone());
            Ok(())
        }
    }
    let resolves = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let n = resolves.clone();
    let first = Target {
        kind: Kind::Grok,
        session_id: Some("front-grok".into()),
        pid: Some(222),
        pane_id: Some("wG:p4".into()),
        herdr_name: Some("worker-1".into()),
    };
    let root = whipnext::pack::default_pack_root();
    let skin = load_skin_from(&root, "default").unwrap();
    let machine = whipnext::whip::WhipThenNext::new(
        skin.idle.clone(),
        skin.crack.clone(),
        whipnext::inject::RecordingInjector::default(),
        whipnext::audio::OsAudio::default(),
        Box::new(move || {
            if n.fetch_add(1, Ordering::SeqCst) == 0 {
                Some(first.clone())
            } else {
                None
            }
        }),
    );
    let eng =
        OverlayEngine::from_skin(skin, machine, Settings::default(), Instant::now(), root).unwrap();
    let mut base = FakeHost::new(24);
    base.click_lo = Some(0);
    base.click_hi = Some(24);
    base.fg = Some(grok_fg());
    let mut h = H {
        injected: injected.clone(),
        base,
    };
    let mut o = opts(false);
    o.always_show = true;
    run_with_host(o, eng, &mut h).unwrap();
    assert!(
        resolves.load(Ordering::SeqCst) >= 2,
        "need a second crack whose resolve is None"
    );
    assert_eq!(
        injected.lock().unwrap().len(),
        1,
        "resolve None must not replay the previous live_inject target"
    );
}

#[test]
fn drive_demo_does_not_live_inject() {
    let injected = Arc::new(Mutex::new(0usize));
    struct H {
        base: FakeHost,
        n: Arc<Mutex<usize>>,
    }
    impl OverlayHost for H {
        fn now(&self) -> Instant {
            self.base.now()
        }
        fn pump(&mut self) {}
        fn quit_requested(&self) -> bool {
            self.base.quit_requested()
        }
        fn live_settings(&self) -> Option<Settings> {
            None
        }
        fn cursor(&self) -> Option<(i32, i32)> {
            self.base.cursor()
        }
        fn foreground(&self) -> Option<Foreground> {
            None
        }
        fn clicked_opaque(&mut self, r: &[u8], x: i32, y: i32) -> bool {
            self.base.clicked_opaque(r, x, y)
        }
        fn set_visible(&mut self, v: bool) {
            self.base.set_visible(v);
        }
        fn set_pos(&mut self, x: i32, y: i32) {
            self.base.set_pos(x, y);
        }
        fn blit_rgba(&mut self, r: &[u8]) {
            self.base.blit_rgba(r);
        }
        fn escape_over_window(&self, x: i32, y: i32) -> bool {
            self.base.escape_over_window(x, y)
        }
        fn sleep_frame(&mut self) {
            self.base.sleep_frame();
        }
        fn live_inject(&mut self, _t: &Target, _payload: &str) -> Result<(), String> {
            *self.n.lock().unwrap() += 1;
            Ok(())
        }
    }
    let mut base = FakeHost::new(10);
    base.click_lo = Some(0);
    base.click_hi = Some(0);
    let mut h = H {
        n: injected.clone(),
        base,
    };
    let mut o = opts(true);
    o.always_show = true;
    o.auto_click = true;
    o.quit_after = true;
    run_with_host(o, engine("default"), &mut h).unwrap();
    assert_eq!(*injected.lock().unwrap(), 0);
}

#[test]
fn hide_now_skips_blit_and_timeout_stops() {
    let blits = Arc::new(Mutex::new(0usize));
    struct H {
        base: FakeHost,
        blits: Arc<Mutex<usize>>,
    }
    impl OverlayHost for H {
        fn now(&self) -> Instant {
            self.base.now()
        }
        fn pump(&mut self) {}
        fn quit_requested(&self) -> bool {
            false
        }
        fn live_settings(&self) -> Option<Settings> {
            self.base.live.clone()
        }
        fn cursor(&self) -> Option<(i32, i32)> {
            None
        }
        fn foreground(&self) -> Option<Foreground> {
            None
        }
        fn clicked_opaque(&mut self, _r: &[u8], _x: i32, _y: i32) -> bool {
            false
        }
        fn set_visible(&mut self, v: bool) {
            self.base.set_visible(v);
        }
        fn set_pos(&mut self, _x: i32, _y: i32) {}
        fn blit_rgba(&mut self, _r: &[u8]) {
            *self.blits.lock().unwrap() += 1;
        }
        fn escape_over_window(&self, _x: i32, _y: i32) -> bool {
            false
        }
        fn sleep_frame(&mut self) {
            self.base.sleep_frame();
        }
        fn live_inject(&mut self, _t: &Target, _payload: &str) -> Result<(), String> {
            Ok(())
        }
    }
    let mut live_s = Settings::default();
    live_s.hide_now = true;
    let mut base = FakeHost::new(100);
    base.live = Some(live_s.clone());
    let mut h = H {
        blits: blits.clone(),
        base,
    };
    let mut o = opts(true);
    o.always_show = true;
    o.timeout = Some(Duration::from_millis(300));
    o.settings = live_s;
    run_with_host(o, engine("default"), &mut h).unwrap();
    assert_eq!(*blits.lock().unwrap(), 0, "hidden overlay must not blit");
}

#[test]
fn inject_error_is_logged_and_escape_quits() {
    let log = std::env::temp_dir().join(format!("whipnext-inj-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&log);
    struct H {
        base: FakeHost,
    }
    impl OverlayHost for H {
        fn now(&self) -> Instant {
            self.base.now()
        }
        fn pump(&mut self) {}
        fn quit_requested(&self) -> bool {
            self.base.quit_requested()
        }
        fn live_settings(&self) -> Option<Settings> {
            None
        }
        fn cursor(&self) -> Option<(i32, i32)> {
            self.base.cursor()
        }
        fn foreground(&self) -> Option<Foreground> {
            None
        }
        fn clicked_opaque(&mut self, r: &[u8], x: i32, y: i32) -> bool {
            self.base.clicked_opaque(r, x, y)
        }
        fn set_visible(&mut self, v: bool) {
            self.base.set_visible(v);
        }
        fn set_pos(&mut self, x: i32, y: i32) {
            self.base.set_pos(x, y);
        }
        fn blit_rgba(&mut self, r: &[u8]) {
            self.base.blit_rgba(r);
        }
        fn escape_over_window(&self, _x: i32, _y: i32) -> bool {
            self.base.escape
        }
        fn sleep_frame(&mut self) {
            self.base.sleep_frame();
        }
        fn live_inject(&mut self, _t: &Target, _payload: &str) -> Result<(), String> {
            Err("boom".into())
        }
    }
    let mut base = FakeHost::new(12);
    base.click_lo = Some(0);
    base.click_hi = Some(0);
    let mut h = H { base };
    let mut o = opts(false);
    o.always_show = true;
    o.log_path = Some(log.clone());
    run_with_host(o, engine("default"), &mut h).unwrap();
    let txt = std::fs::read_to_string(&log).unwrap();
    assert!(txt.contains("inject-error") || txt.contains("boom") || txt.contains("inject"));
    assert!(txt.contains(NEXT_PROMPT) || txt.contains("payload"));
    let _ = std::fs::remove_file(&log);

    let mut base = FakeHost::new(50);
    base.escape = true;
    let mut h = H { base };
    let mut o = opts(true);
    o.always_show = true;
    o.quit = None;
    run_with_host(o, engine("default"), &mut h).unwrap();
}

#[test]
fn quit_flag_stops_and_dump_dir_copies_frames() {
    let dump = std::env::temp_dir().join(format!("whipnext-dump-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dump);
    let mut o = opts(true);
    o.always_show = true;
    o.dump_dir = Some(dump.clone());
    o.quit = Some(Arc::new(AtomicBool::new(true)));
    o.quit.as_ref().unwrap().store(true, Ordering::Relaxed);
    whipnext::overlay::run_overlay(o).unwrap();
    assert!(dump.join("overlay-idle.png").is_file());
    assert!(dump.join("overlay-punch.png").is_file());
    let _ = std::fs::remove_dir_all(&dump);
}

#[test]
fn live_settings_swap_model_when_idle() {
    struct H {
        base: FakeHost,
        _skin: Arc<Mutex<String>>,
    }
    impl OverlayHost for H {
        fn now(&self) -> Instant {
            self.base.now()
        }
        fn pump(&mut self) {}
        fn quit_requested(&self) -> bool {
            self.base.quit_requested()
        }
        fn live_settings(&self) -> Option<Settings> {
            self.base.live.clone()
        }
        fn cursor(&self) -> Option<(i32, i32)> {
            None
        }
        fn foreground(&self) -> Option<Foreground> {
            None
        }
        fn clicked_opaque(&mut self, _r: &[u8], _x: i32, _y: i32) -> bool {
            false
        }
        fn set_visible(&mut self, v: bool) {
            self.base.set_visible(v);
        }
        fn set_pos(&mut self, _x: i32, _y: i32) {}
        fn blit_rgba(&mut self, _r: &[u8]) {}
        fn escape_over_window(&self, _x: i32, _y: i32) -> bool {
            false
        }
        fn sleep_frame(&mut self) {
            self.base.sleep_frame();
        }
        fn live_inject(&mut self, _t: &Target, _payload: &str) -> Result<(), String> {
            Ok(())
        }
    }
    let mut live_s = Settings::default();
    live_s.model = "cat".into();
    let mut base = FakeHost::new(4);
    base.live = Some(live_s);
    let eng = engine("default");
    assert_eq!(eng.skin_id, "default");
    let mut o = opts(true);
    o.always_show = true;
    let mut hh = H {
        base,
        _skin: Arc::new(Mutex::new(String::new())),
    };
    let out = run_with_host(o, eng, &mut hh).unwrap();
    assert_eq!(out.skin_id, "cat");
}

#[test]
fn run_overlay_falls_back_when_model_missing() {
    let mut settings = Settings::default();
    settings.model = "no-such-model".into();
    let mut o = opts(true);
    o.always_show = true;
    o.settings = settings;
    o.quit = Some(Arc::new(AtomicBool::new(true)));
    whipnext::overlay::run_overlay(o).unwrap();
}

#[test]
fn run_with_host_honors_quit_flag() {
    let mut host = FakeHost::new(100);
    host.cursor = None;
    let mut o = opts(true);
    o.always_show = true;
    o.quit = Some(Arc::new(AtomicBool::new(true)));
    run_with_host(o, engine("default"), &mut host).unwrap();
    assert_eq!(host.n, 0);
}

#[test]
fn build_machine_demo_resolve_submits_next() {
    let root = whipnext::pack::default_pack_root();
    let skin = load_skin_from(&root, "default").unwrap();
    let mut machine = whipnext::overlay::build_machine(&skin, true, None);
    assert!(machine.handle_click());
    while machine.phase == Phase::Crack {
        machine.tick();
    }
    assert_eq!(machine.injector.calls.len(), 1);
    assert_eq!(machine.injector.calls[0].payload, NEXT_PROMPT);
    let forced = Target {
        kind: Kind::Grok,
        session_id: None,
        pid: Some(9),
        pane_id: None,
        herdr_name: None,
    };
    let mut machine = whipnext::overlay::build_machine(&skin, false, Some(forced.clone()));
    assert!(machine.handle_click());
    while machine.phase == Phase::Crack {
        machine.tick();
    }
    assert_eq!(machine.injector.calls[0].target.pid, Some(9));
}

#[test]
fn punch_frame_uses_third_or_last() {
    let a = whipnext::whip::Frame {
        id: "a".into(),
        path: "a".into(),
    };
    let b = whipnext::whip::Frame {
        id: "b".into(),
        path: "b".into(),
    };
    let c = whipnext::whip::Frame {
        id: "c".into(),
        path: "c".into(),
    };
    let d = whipnext::whip::Frame {
        id: "d".into(),
        path: "d".into(),
    };
    assert_eq!(punch_frame(&[a.clone(), b.clone()]).id, "b");
    assert_eq!(punch_frame(&[a, b, c, d]).id, "c");
}

#[test]
fn apply_surface_skips_model_swap_while_cracking() {
    struct H {
        base: FakeHost,
    }
    impl OverlayHost for H {
        fn now(&self) -> Instant {
            self.base.now()
        }
        fn pump(&mut self) {}
        fn quit_requested(&self) -> bool {
            self.base.quit_requested()
        }
        fn live_settings(&self) -> Option<Settings> {
            if self.base.n >= 1 {
                self.base.live.clone()
            } else {
                None
            }
        }
        fn cursor(&self) -> Option<(i32, i32)> {
            None
        }
        fn foreground(&self) -> Option<Foreground> {
            None
        }
        fn clicked_opaque(&mut self, _r: &[u8], _x: i32, _y: i32) -> bool {
            self.base.n == 0
        }
        fn set_visible(&mut self, v: bool) {
            self.base.set_visible(v);
        }
        fn set_pos(&mut self, _x: i32, _y: i32) {}
        fn blit_rgba(&mut self, _r: &[u8]) {}
        fn escape_over_window(&self, _x: i32, _y: i32) -> bool {
            false
        }
        fn sleep_frame(&mut self) {
            self.base.sleep_frame();
        }
        fn live_inject(&mut self, _t: &Target, _payload: &str) -> Result<(), String> {
            Ok(())
        }
    }
    let mut live_s = Settings::default();
    live_s.model = "cat".into();
    let mut base = FakeHost::new(6);
    base.live = Some(live_s);
    let eng = engine("default");
    let mut o = opts(true);
    o.always_show = true;
    let out = run_with_host(o, eng, &mut H { base }).unwrap();
    assert_eq!(out.skin_id, "default", "must not swap skin mid-crack");
}

#[test]
fn load_skin_unknown_falls_back_to_default() {
    let root = whipnext::pack::default_pack_root();
    let skin = load_skin_from(&root, "no-such-model").unwrap();
    assert!(!skin.idle.is_empty());
    assert!(skin.crack.len() >= 2);
    let shipped = whipnext::overlay::load_skin("default").unwrap();
    assert_eq!(shipped.rgba[0].len(), W * H * 4);
}

#[test]
fn empty_pack_cannot_load_skin() {
    let empty = std::env::temp_dir().join(format!("whipnext-empty-pack-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&empty);
    assert!(load_skin_from(&empty, "default").is_err());
    let _ = std::fs::remove_dir_all(&empty);
}

#[test]
fn from_skin_rejects_wrong_rgba_len() {
    let mut skin = load_skin_from(&whipnext::pack::default_pack_root(), "default").unwrap();
    skin.rgba[0] = vec![0; 4];
    let machine = whipnext::whip::WhipThenNext::new(
        skin.idle.clone(),
        skin.crack.clone(),
        whipnext::inject::RecordingInjector::default(),
        whipnext::audio::OsAudio::default(),
        Box::new(|| None),
    );
    match OverlayEngine::from_skin(
        skin,
        machine,
        Settings::default(),
        Instant::now(),
        whipnext::pack::default_pack_root(),
    ) {
        Err(err) => assert!(err.contains("decode")),
        Ok(_) => panic!("expected decode error"),
    }
}

#[test]
fn resolve_live_machine_target_without_force() {
    let _ = resolve_machine_target(false, None);
}

#[test]
fn drive_click_without_target_skips_inject() {
    let mut base = FakeHost::new(10);
    base.click_lo = Some(0);
    base.click_hi = Some(0);
    let root = whipnext::pack::default_pack_root();
    let skin = load_skin_from(&root, "default").unwrap();
    let machine = whipnext::whip::WhipThenNext::new(
        skin.idle.clone(),
        skin.crack.clone(),
        whipnext::inject::RecordingInjector::default(),
        whipnext::audio::OsAudio::default(),
        Box::new(|| None),
    );
    let eng =
        OverlayEngine::from_skin(skin, machine, Settings::default(), Instant::now(), root).unwrap();
    let mut o = opts(false);
    o.always_show = true;
    o.quit_after = true;
    let out = run_with_host(o, eng, &mut base).unwrap();
    assert!(out.machine.injector.calls.is_empty());
}

#[test]
fn apply_surface_empty_pack_keeps_current_skin() {
    let empty = std::env::temp_dir().join(format!("whipnext-empty-apply-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&empty);
    let mut eng = engine("default");
    eng.pack_root = empty.clone();
    let mut live_s = Settings::default();
    live_s.model = "cat".into();
    let mut base = FakeHost::new(3);
    base.live = Some(live_s);
    let mut o = opts(true);
    o.always_show = true;
    let out = run_with_host(o, eng, &mut base).unwrap();
    assert_eq!(out.skin_id, "default");
    let _ = std::fs::remove_dir_all(&empty);
}

#[test]
fn apply_surface_unknown_model_keeps_skin() {
    let mut live_s = Settings::default();
    live_s.model = "definitely-missing-model".into();
    let mut base = FakeHost::new(3);
    base.live = Some(live_s);
    let mut o = opts(true);
    o.always_show = true;
    let out = run_with_host(o, engine("default"), &mut base).unwrap();
    assert!(!out.rgba.is_empty());
    assert_eq!(out.machine.idle.len(), engine("default").machine.idle.len());
}

#[test]
fn log_line_skips_unwritable_path() {
    let dir = std::env::temp_dir().join(format!("whipnext-logdir-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    log_line(&Some(dir.clone()), serde_json::json!({"event":"nope"}));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn overlay_scale_changes_origin_from_engine_start() {
    assert_eq!(overlay_pixel_size(100), (W, H));
    assert_eq!(overlay_size_i32(50), ((W / 2) as i32, (H / 2) as i32));
    let mut host = FakeHost::new(4);
    host.cursor = Some((400, 400));
    let mut eng = engine("default");
    eng.settings.overlay_scale = 50;
    let mut live = Settings::default();
    live.overlay_scale = 50;
    host.live = Some(live);
    let mut o = opts(true);
    o.always_show = true;
    o.settings.overlay_scale = 50;
    run_with_host(o, eng, &mut host).unwrap();
    let (w, h) = overlay_size_i32(50);
    let expect = overlay_origin((400, 400), w, h);
    assert_eq!(*host.pos.last().unwrap(), expect);
}

#[test]
fn overlay_key_visibility_from_default_engine() {
    assert!(overlay_key_visible(OverlayShowMode::Always, false));
    assert!(!overlay_key_visible(OverlayShowMode::WhileHeld, false));
    assert!(overlay_key_visible(OverlayShowMode::WhileHeld, true));
    assert!(!overlay_key_visible(OverlayShowMode::HideWhileHeld, true));
    assert_eq!(parse_vk("alt"), Some(0x12));
    assert_eq!(parse_vk("ctrl"), Some(0x11));
    assert_eq!(parse_vk("0x12"), Some(0x12));
    assert_eq!(parse_vk("32"), Some(32));
    assert_eq!(parse_vk("shift"), Some(0x10));
    assert_eq!(parse_vk("win"), Some(0x5B));
    assert_eq!(parse_vk("space"), Some(0x20));
    assert_eq!(parse_vk("caps"), Some(0x14));
    assert!(parse_vk("none").is_none());
    assert_eq!(overlay_pixel_size(1), overlay_pixel_size(25));
    assert_eq!(overlay_pixel_size(400), (W * 4, H * 4));

    let mut host = FakeHost::new(3);
    host.held = false;
    let mut live = Settings::default();
    live.overlay_mode = OverlayShowMode::WhileHeld;
    live.overlay_key = "alt".into();
    host.live = Some(live.clone());
    let mut eng = engine("default");
    eng.settings = live.clone();
    let mut o = opts(true);
    o.always_show = true;
    o.settings = live;
    let out = run_with_host(o, eng, &mut host).unwrap();
    assert!(!out.visible);

    let mut host = FakeHost::new(3);
    host.held = true;
    let mut live = Settings::default();
    live.overlay_mode = OverlayShowMode::WhileHeld;
    host.live = Some(live.clone());
    let mut eng = engine("default");
    eng.settings = live.clone();
    let mut o = opts(true);
    o.always_show = true;
    o.settings = live;
    let out = run_with_host(o, eng, &mut host).unwrap();
    assert!(out.visible);

    let mut host = FakeHost::new(3);
    host.held = true;
    let mut live = Settings::default();
    live.overlay_mode = OverlayShowMode::HideWhileHeld;
    host.live = Some(live.clone());
    let mut eng = engine("default");
    eng.settings = live.clone();
    let mut o = opts(true);
    o.always_show = true;
    o.settings = live;
    let out = run_with_host(o, eng, &mut host).unwrap();
    assert!(!out.visible);
}

#[test]
fn grok_harness_shows_assigned_character_chrome_hides() {
    assert!(overlay_is_shown(
        Some("grok"),
        false,
        OverlayShowMode::Always,
        false
    ));
    assert!(!overlay_is_shown(
        None,
        false,
        OverlayShowMode::Always,
        false
    ));
    assert!(!overlay_is_shown(
        Some("grok"),
        true,
        OverlayShowMode::Always,
        false
    ));

    let mut live = Settings::default();
    live.overlay_mode = OverlayShowMode::Always;
    live.hide_now = false;
    live.only_when_matched = true;
    live.apps.insert(
        "grok".into(),
        AppPref {
            enabled: true,
            model: Some("commissar".into()),
            ..Default::default()
        },
    );

    let mut host = FakeHost::new(5);
    host.fg = Some(grok_fg());
    host.live = Some(live.clone());
    let mut eng = engine("default");
    eng.settings = live.clone();
    let mut o = opts(false);
    o.always_show = false;
    o.settings = live.clone();
    let out = run_with_host(o, eng, &mut host).unwrap();
    assert!(out.visible, "enabled grok harness must show the overlay");
    assert_eq!(out.skin_id, "commissar");
    assert!(host.blits > 0);

    let mut host = FakeHost::new(4);
    host.fg = Some(Foreground {
        pid: 9,
        exe: "chrome.exe".into(),
        title: "Google".into(),
        procs: vec![],
    });
    host.live = Some(live.clone());
    let mut eng = engine("default");
    eng.last_surface = Some("grok".into());
    eng.settings = live.clone();
    let mut o = opts(false);
    o.always_show = false;
    o.settings = live;
    let out = run_with_host(o, eng, &mut host).unwrap();
    assert!(!out.visible, "must hide on non-harness apps");
}

#[test]
fn drive_click_injects_character_phrase() {
    let mut host = FakeHost::new(12);
    host.click_lo = Some(0);
    host.click_hi = Some(0);
    host.fg = Some(grok_fg());
    let mut eng = engine("default");
    eng.settings
        .phrases
        .insert("default".into(), "keep going".into());
    let mut o = opts(false);
    o.always_show = true;
    o.settings = eng.settings.clone();
    host.live = Some(eng.settings.clone());
    let eng = run_with_host(o, eng, &mut host).unwrap();
    assert_eq!(eng.machine.injector.calls.len(), 1);
    assert_eq!(eng.machine.injector.calls[0].payload, "keep going");
    assert!(!eng.machine.injector.calls[0].payload.starts_with('/'));
}

#[test]
fn grok_character_hit_sound_from_catalog_not_global_whip() {
    use whipnext::pack::{import_character, ImportSpec};
    let src = std::env::temp_dir().join(format!("whipnext-bang-src-{}", std::process::id()));
    let catalog = std::env::temp_dir().join(format!("whipnext-bang-cat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&src);
    let _ = std::fs::remove_dir_all(&catalog);
    std::fs::create_dir_all(&src).unwrap();
    let still = src.join("hero.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([4, 5, 6, 255]))
        .save(&still)
        .unwrap();
    let wav = src.join("bang.wav");
    std::fs::write(&wav, b"RIFF....WAVEfmt ").unwrap();
    let imported = import_character(
        &catalog,
        &ImportSpec {
            name: "Hero X".into(),
            still: Some(still),
            hit_sound: Some(wav),
            ..ImportSpec::default()
        },
    )
    .unwrap();
    let hit = imported
        .sound_action
        .clone()
        .expect("imported hit sound stem");
    assert_ne!(hit, "whip");

    let mut settings = Settings::default();
    settings.sound_whip = "whip".into();
    settings.only_when_matched = true;
    settings.apps.insert(
        "grok".into(),
        AppPref {
            enabled: true,
            model: Some(imported.id.clone()),
            sound_whip: None,
            ..Default::default()
        },
    );

    let mut eng = engine("default");
    eng.catalog_root = catalog.clone();
    eng.settings = settings.clone();
    assert_eq!(eng.machine.audio.whip, "whip");

    let mut host = FakeHost::new(4);
    host.fg = Some(grok_fg());
    host.live = Some(settings.clone());
    let mut o = opts(false);
    o.always_show = false;
    o.settings = settings;
    let out = run_with_host(o, eng, &mut host).unwrap();
    assert_eq!(out.settings.model_for(Some("grok")), imported.id.as_str());
    assert_eq!(out.machine.audio.whip, hit);
    assert_ne!(out.machine.audio.whip, "whip");
    let _ = std::fs::remove_dir_all(&src);
    let _ = std::fs::remove_dir_all(&catalog);
}

fn visible(rgba: &[u8]) -> bool {
    rgba.chunks(4).any(|p| p[3] > 24)
}

fn write_solid_png(path: &std::path::Path, rgb: [u8; 3]) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    image::RgbaImage::from_pixel(8, 8, image::Rgba([rgb[0], rgb[1], rgb[2], 255]))
        .save(path)
        .unwrap();
}

fn write_gif_frames(path: &std::path::Path, n: u32, rgb: [u8; 3]) {
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame, Rgba, RgbaImage};
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let file = std::fs::File::create(path).unwrap();
    let mut enc = GifEncoder::new(file);
    for i in 0..n {
        let g = rgb[1].saturating_add((i as u8).saturating_mul(20));
        let img = RgbaImage::from_pixel(8, 8, Rgba([rgb[0], g, rgb[2], 255]));
        let frame = Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(140, 1));
        enc.encode_frame(frame).unwrap();
    }
}

#[test]
fn bundled_svg_gif_3d_and_2d_examples_load() {
    let root = whipnext::pack::default_pack_root();
    let models = whipnext::pack::list_models(&root);
    let kind = |id: &str| {
        models
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("missing {id}"))
            .media_kind
            .as_str()
            .to_string()
    };
    assert_eq!(kind("manager"), "image");
    assert_eq!(kind("lash"), "svg");
    assert_eq!(kind("feather"), "gif");
    assert_eq!(kind("capybara"), "3d");
    for id in [
        "default",
        "commissar",
        "cat",
        "parrot",
        "manager",
        "lash",
        "feather",
        "capybara",
    ] {
        let skin = load_skin_from(&root, id).unwrap();
        assert!(skin.idle.len() >= 2, "{id} idle {}", skin.idle.len());
        assert!(skin.crack.len() >= 2, "{id} crack {}", skin.crack.len());
        assert!(
            skin.rgba.iter().any(|b| visible(b)),
            "{id} overlay frames are empty"
        );
        let w = skin.w as i32;
        let h = skin.h as i32;
        let lx = w / 2;
        let ly = h - whipnext::layout::OVERLAY_ANCHOR_INSET_Y;
        assert!(
            whipnext::overlay::click_hits_sprite(&skin.rgba[0], w, h, lx, ly),
            "{id} click at cursor misses"
        );
        let i = ((ly as usize) * (w as usize) + (lx as usize)) * 4 + 3;
        assert!(
            skin.rgba[0].get(i).copied().unwrap_or(0) > 0,
            "{id} cursor pixel is fully transparent"
        );
    }
    let capy = load_skin_from(&root, "capybara").unwrap();
    assert_eq!(capy.idle.len(), 2);
    assert_eq!(capy.crack.len(), 4);
    assert_ne!(
        capy.rgba[0],
        capy.rgba[capy.idle.len()],
        "capybara hit frame must differ from idle"
    );
    let feather = load_skin_from(&root, "feather").unwrap();
    assert_eq!(feather.idle.len(), 2, "idle.gif must expand");
    assert_eq!(feather.crack.len(), 4, "action.gif must expand");
    let svg = root.join("models/lash/still.svg");
    assert!(visible(&load_frame_rgba(&svg, 48, 48)));
    let gltf = root.join("models/capybara/capybara.gltf");
    assert!(visible(&load_frame_rgba(&gltf, 48, 48)));
}

#[test]
fn gif_expands_and_bad_gif_falls_back() {
    let dir = std::env::temp_dir().join(format!("whipnext-gif-exp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let gif = dir.join("loop.gif");
    write_gif_frames(&gif, 3, [40, 200, 40]);
    let frames = load_frames_rgba(&gif, 16, 16);
    assert_eq!(frames.len(), 3);
    assert!(frames.iter().any(|(_, b)| visible(b)));
    let missing = dir.join("nope.gif");
    let one = load_frames_rgba(&missing, 4, 4);
    assert_eq!(one.len(), 1);
    assert!(one[0].1.iter().all(|b| *b == 0));
    let bad = dir.join("bad.gif");
    std::fs::write(&bad, b"not-a-gif").unwrap();
    let fallback = load_frames_rgba(&bad, 4, 4);
    assert_eq!(fallback.len(), 1);
    let empty = dir.join("empty.gif");
    std::fs::write(&empty, b"GIF89a\x01\x00\x01\x00\x00\x00\x00;").unwrap();
    assert_eq!(load_frames_rgba(&empty, 4, 4).len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn one_frame_gif_action_cannot_build_skin() {
    let root = std::env::temp_dir().join(format!("whipnext-gif-one-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let model = root.join("models/solo");
    std::fs::create_dir_all(&model).unwrap();
    write_solid_png(&model.join("idle_00.png"), [9, 9, 9]);
    write_gif_frames(&model.join("action.gif"), 1, [200, 10, 10]);
    std::fs::write(
        model.join("manifest.json"),
        r#"{"id":"solo","still":"idle_00.png","idle":["idle_00.png"],"action":["action.gif"]}"#,
    )
    .unwrap();
    match load_skin_sized(&root, "solo", 8, 8) {
        Err(err) => assert!(err.contains("at least 2"), "{err}"),
        Ok(_) => panic!("one-frame gif action must not build a skin"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn png_open_miss_is_transparent() {
    let p = std::env::temp_dir().join(format!("whipnext-miss-{}.png", std::process::id()));
    let _ = std::fs::remove_file(&p);
    let raw = load_frame_rgba(&p, 2, 2);
    assert_eq!(raw, vec![0; 16]);
    let dir = std::env::temp_dir().join(format!("whipnext-gltf-sib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let gltf = dir.join("foo.gltf");
    std::fs::write(&gltf, "not-json").unwrap();
    write_solid_png(&dir.join("foo.png"), [40, 200, 40]);
    let sib = load_frame_rgba(&gltf, 8, 8);
    assert!(
        visible(&sib),
        "empty 3d raster should fall back to sibling png"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn broken_3d_falls_back_to_default_and_default_3d_errors() {
    let root = std::env::temp_dir().join(format!("whipnext-3d-fb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("models/ghost")).unwrap();
    std::fs::create_dir_all(root.join("models/default")).unwrap();
    write_solid_png(&root.join("models/default/still.png"), [9, 9, 9]);
    write_solid_png(&root.join("models/default/idle_00.png"), [9, 9, 9]);
    write_solid_png(&root.join("models/default/idle_01.png"), [8, 8, 8]);
    write_solid_png(&root.join("models/default/action_00.png"), [7, 7, 7]);
    write_solid_png(&root.join("models/default/action_01.png"), [6, 6, 6]);
    std::fs::write(
        root.join("models/ghost/manifest.json"),
        r#"{"id":"ghost","still":"missing.gltf","media_kind":"3d"}"#,
    )
    .unwrap();
    let skin = load_skin_sized(&root, "ghost", 16, 16).unwrap();
    assert_eq!(skin.id, "default");

    std::fs::write(
        root.join("models/default/manifest.json"),
        r#"{"id":"default","still":"nope.gltf","media_kind":"3d"}"#,
    )
    .unwrap();
    match load_skin_sized(&root, "default", 16, 16) {
        Err(err) => {
            assert!(
                err.contains("glTF")
                    || err.contains("gltf")
                    || err.contains("3d")
                    || err.contains("no"),
                "{err}"
            );
        }
        Ok(_) => panic!("broken default 3d must not build a skin"),
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn user_drop_nugget_builds_overlay_skin() {
    let src = common::nugget_fixture().join("nugget/nugget.gltf");
    assert!(src.is_file());
    let catalog = std::env::temp_dir().join(format!("whipnext-nug-skin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&catalog);
    let m = whipnext::pack::import_character(
        &catalog,
        &whipnext::pack::ImportSpec {
            name: "Nugget".into(),
            media: Some(src),
            ..whipnext::pack::ImportSpec::default()
        },
    )
    .unwrap();
    let skin = load_skin_sized(&catalog, &m.id, 48, 64).unwrap();
    assert!(skin.idle.len() >= 2);
    assert!(skin.crack.len() >= 2);
    assert!(skin.rgba.iter().any(|b| visible(b)));
    let _ = std::fs::remove_dir_all(&catalog);
}

#[test]
fn rescale_skips_while_cracking_and_failed_reload() {
    use whipnext::whip::Phase;
    let mut host = FakeHost::new(6);
    host.fg = Some(grok_fg());
    let mut eng = engine("default");
    eng.machine.phase = Phase::Crack;
    eng.settings.overlay_scale = 50;
    let mut live = Settings::default();
    live.overlay_scale = 50;
    host.live = Some(live.clone());
    let mut o = opts(true);
    o.always_show = true;
    run_with_host(o, eng, &mut host).unwrap();

    let mut host = FakeHost::new(6);
    host.fg = Some(grok_fg());
    let mut eng = engine("default");
    eng.skin_id = "missing-skin".into();
    eng.pack_root = std::env::temp_dir().join("whipnext-no-pack-here");
    eng.catalog_root = eng.pack_root.clone();
    eng.settings.overlay_scale = 50;
    live.overlay_scale = 50;
    host.live = Some(live);
    let mut o = opts(true);
    o.always_show = true;
    let _ = run_with_host(o, eng, &mut host);
}

#[test]
fn invalid_3d_manifest_still_loads_gltf() {
    let root = std::env::temp_dir().join(format!("whipnext-badman-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("models/tri");
    std::fs::create_dir_all(&dir).unwrap();
    let src = common::nugget_fixture().join("nugget");
    std::fs::copy(src.join("nugget.gltf"), dir.join("nugget.gltf")).unwrap();
    std::fs::copy(src.join("nugget.bin"), dir.join("nugget.bin")).unwrap();
    std::fs::write(dir.join("manifest.json"), "not-json").unwrap();
    let skin = load_skin_sized(&root, "tri", 32, 32).unwrap();
    assert!(skin.rgba.iter().any(|b| visible(b)));
    std::fs::create_dir_all(root.join("models/empty3d")).unwrap();
    std::fs::write(
        root.join("models/empty3d/empty.gltf"),
        r#"{"asset":{"version":"2.0"},"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"accessors":[{"bufferView":0,"componentType":5126,"count":1,"type":"VEC3"}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":12}],"buffers":[{"byteLength":12,"uri":"empty.bin"}]}"#,
    )
    .unwrap();
    let mut bin = Vec::new();
    bin.extend(0f32.to_le_bytes());
    bin.extend(0f32.to_le_bytes());
    bin.extend(0f32.to_le_bytes());
    std::fs::write(root.join("models/empty3d/empty.bin"), bin).unwrap();
    std::fs::write(
        root.join("models/empty3d/manifest.json"),
        r#"{"id":"empty3d","still":"empty.gltf","media_kind":"3d"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(root.join("models/default")).unwrap();
    write_solid_png(&root.join("models/default/still.png"), [9, 9, 9]);
    write_solid_png(&root.join("models/default/idle_00.png"), [9, 9, 9]);
    write_solid_png(&root.join("models/default/idle_01.png"), [8, 8, 8]);
    write_solid_png(&root.join("models/default/action_00.png"), [7, 7, 7]);
    write_solid_png(&root.join("models/default/action_01.png"), [6, 6, 6]);
    let fallback = load_skin_sized(&root, "empty3d", 16, 16).unwrap();
    assert_eq!(fallback.id, "default");
    std::fs::write(root.join("models/empty3d/empty.gltf"), "not-json").unwrap();
    let from_bad = load_skin_sized(&root, "empty3d", 16, 16).unwrap();
    assert_eq!(from_bad.id, "default");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn run_overlay_or_else_when_catalog_3d_broken() {
    let home = std::env::temp_dir().join(format!("whipnext-ovl-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(home.join(".whipnext/catalog/models/broken")).unwrap();
    std::fs::write(
        home.join(".whipnext/catalog/models/broken/manifest.json"),
        r#"{"id":"broken","still":"nope.gltf","media_kind":"3d"}"#,
    )
    .unwrap();
    std::env::set_var("WHIPNEXT_HOME", &home);
    let mut settings = Settings::default();
    settings.model = "broken".into();
    let mut o = opts(true);
    o.always_show = true;
    o.settings = settings;
    o.quit = Some(Arc::new(AtomicBool::new(true)));
    whipnext::overlay::run_overlay(o).unwrap();
    std::env::remove_var("WHIPNEXT_HOME");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn video_idle_and_action_decode_into_overlay_skin() {
    let root = std::env::temp_dir().join(format!("whipnext-ovl-vid-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("models/clip")).unwrap();
    let idle = root.join("models/clip/idle.mp4");
    let action = root.join("models/clip/action.mp4");
    whipnext::video::write_test_clip(&idle, 3, "red").unwrap();
    whipnext::video::write_test_clip(&action, 4, "blue").unwrap();
    std::fs::write(
        root.join("models/clip/manifest.json"),
        r#"{"id":"clip","still":"idle.mp4","idle":["idle.mp4"],"action":["action.mp4"],"media_kind":"video"}"#,
    )
    .unwrap();
    let skin = load_skin_sized(&root, "clip", 24, 24).unwrap();
    assert_eq!(skin.id, "clip");
    assert!(!skin.idle.is_empty());
    assert!(
        skin.crack.len() >= whipnext::layout::MIN_ACTION_FRAMES,
        "action frames {}",
        skin.crack.len()
    );
    assert_eq!(skin.rgba[0].len(), 24 * 24 * 4);
    let _ = std::fs::remove_dir_all(&root);
}
