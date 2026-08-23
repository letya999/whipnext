//! Settings window (WebView) + overlay thread. Closing the window stops the overlay.

use std::borrow::Cow;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
#[cfg(windows)]
use tao::platform::windows::WindowExtWindows;
use tao::window::WindowBuilder;
use wry::http::header::CONTENT_TYPE;
use wry::http::Response;
use wry::WebViewBuilder;

use crate::os_probe::SystemProbe;
use crate::overlay::{self, LaunchOpts};
use crate::pack;
use crate::settings;
use crate::ui::{self, Ipc};

fn reveal_in_file_manager(path: &std::path::Path) {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("explorer.exe");
        c.arg(path);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    let _ = cmd.spawn();
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    #[link_name = "GetModuleHandleW"]
    fn get_module_handle(name: *const u16) -> isize;
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    #[link_name = "LoadImageW"]
    fn load_image(
        instance: isize,
        name: *const u16,
        kind: u32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> isize;
    #[link_name = "SendMessageW"]
    fn send_message(hwnd: isize, message: u32, wparam: usize, lparam: isize) -> isize;
    #[link_name = "SetClassLongPtrW"]
    fn set_class_long_ptr(hwnd: isize, index: i32, value: isize) -> isize;
}

#[cfg(windows)]
fn set_native_window_icon(hwnd: isize) {
    const IMAGE_ICON: u32 = 1;
    const WM_SETICON: u32 = 0x0080;
    const GCLP_HICON: i32 = -14;
    const GCLP_HICONSM: i32 = -34;
    unsafe {
        let module = get_module_handle(std::ptr::null());
        let resource = std::ptr::without_provenance(1);
        let big = load_image(module, resource, IMAGE_ICON, 32, 32, 0);
        let small = load_image(module, resource, IMAGE_ICON, 16, 16, 0);
        if big != 0 {
            send_message(hwnd, WM_SETICON, 1, big);
            set_class_long_ptr(hwnd, GCLP_HICON, big);
        }
        if small != 0 {
            send_message(hwnd, WM_SETICON, 0, small);
            send_message(hwnd, WM_SETICON, 2, small);
            set_class_long_ptr(hwnd, GCLP_HICONSM, small);
        }
    }
}

fn home() -> std::path::PathBuf {
    settings::home_dir()
}

fn launch_log(msg: &str) {
    ui::write_launch_log(&home(), msg);
}

fn wry_asset(body: ui::AssetBody) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(body.status)
        .header(CONTENT_TYPE, body.mime)
        .header("Cache-Control", "no-cache")
        .header("Access-Control-Allow-Origin", "*")
        .body(Cow::Owned(body.bytes))
        .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b""[..])))
}

pub fn run() -> Result<(), String> {
    launch_log("run() start");
    let path = settings::settings_path(&home());
    let mut initial = settings::load_or_propose(&path, ui::proposed_from(&SystemProbe));
    if std::env::args().any(|a| a == "--dev") {
        initial.dev_mode = true;
    }
    let live = Arc::new(Mutex::new(initial.clone()));
    let quit = Arc::new(AtomicBool::new(false));

    {
        let live = live.clone();
        let quit = quit.clone();
        std::thread::spawn(move || {
            let snap = live.lock().ok().map(|g| g.clone()).unwrap_or_default();
            let opts = LaunchOpts {
                demo: false,
                auto_click: false,
                quit_after: false,
                log_path: Some(home().join(".whipnext").join("overlay.log")),
                timeout: None,
                dump_dir: None,
                force_target: None,
                always_show: false,
                settings: snap,
                live: Some(live),
                quit: Some(quit),
            };
            let result = catch_unwind(AssertUnwindSafe(|| overlay::run_overlay(opts)));
            match result {
                Ok(Ok(())) => launch_log("overlay exit ok"),
                Ok(Err(e)) => launch_log(&format!("overlay err: {e}")),
                Err(_) => launch_log("overlay panicked"),
            }
        });
    }

    let event_loop = EventLoop::new();
    let mut wb = WindowBuilder::new()
        .with_title("whipnext")
        .with_maximized(true)
        .with_inner_size(tao::dpi::LogicalSize::new(920.0, 940.0));
    let icon_path = crate::audio::assets_dir().join("icon-256.png");
    if let Ok(img) = image::open(&icon_path) {
        let rgba = img.into_rgba8();
        let (w, h) = rgba.dimensions();
        if let Ok(icon) = tao::window::Icon::from_rgba(rgba.into_raw(), w, h) {
            wb = wb.with_window_icon(Some(icon));
        }
    }
    let window = wb.build(&event_loop).map_err(|e| {
        launch_log(&format!("window: {e}"));
        e.to_string()
    })?;
    #[cfg(windows)]
    set_native_window_icon(window.hwnd());
    launch_log("window ok");

    let assets = crate::audio::assets_dir();
    let live_ipc = live.clone();
    let path_ipc = path.clone();
    let quit_ipc = quit.clone();
    let boot = ui::snapshot_merged(
        &initial,
        &pack::default_pack_root(),
        &pack::catalog_dir(&home()),
    )
    .to_string();
    let pending = Arc::new(Mutex::new(None::<String>));
    let pending_ipc = pending.clone();
    let webview = WebViewBuilder::new()
        .with_initialization_script(format!("window.__BOOT = {boot};"))
        .with_custom_protocol("asset".into(), move |_id, request| {
            wry_asset(ui::serve_asset_ex(
                &assets,
                &pack::catalog_dir(&home()),
                request.uri().path(),
            ))
        })
        .with_url("http://asset.localhost/ui/index.html")
        .with_ipc_handler(move |req| {
            let msg = req.body();
            launch_log(&format!(
                "ipc {}",
                &msg.chars().take(80).collect::<String>()
            ));
            let ipc = ui::parse_ipc(msg);
            match ipc {
                Ipc::Ready
                | Ipc::Set(_)
                | Ipc::Redetect
                | Ipc::Import(_)
                | Ipc::ImportBytes { .. }
                | Ipc::ImportSound { .. }
                | Ipc::ReplaceBytes { .. }
                | Ipc::Rename { .. } => {
                    if let Some(s) =
                        ui::apply_ipc(ipc, &path_ipc, &live_ipc, ui::proposed_from(&SystemProbe))
                    {
                        let snap = ui::snapshot_merged(
                            &s,
                            &pack::default_pack_root(),
                            &pack::catalog_dir(&home()),
                        );
                        if let Ok(mut g) = pending_ipc.lock() {
                            *g = Some(format!("window.__whipnext_set({snap});"));
                        }
                    }
                }
                Ipc::OpenCharacterFolder(id) => {
                    let catalog_dir = pack::catalog_dir(&home());
                    let catalog_model = catalog_dir.join("models").join(&id);
                    let bundled_model = pack::default_pack_root().join("models").join(&id);
                    let target = if catalog_model.is_dir() {
                        catalog_model
                    } else if bundled_model.is_dir() {
                        bundled_model
                    } else {
                        catalog_dir
                    };
                    let _ = std::fs::create_dir_all(&target);
                    reveal_in_file_manager(&target);
                }
                Ipc::OpenSoundFolder => {
                    let catalog = pack::catalog_dir(&home());
                    let settings = live
                        .lock()
                        .ok()
                        .map(|value| value.clone())
                        .unwrap_or_default();
                    let model = pack::list_models_merged(&pack::default_pack_root(), &catalog)
                        .into_iter()
                        .find(|model| model.id == settings.model);
                    let stem = settings
                        .model_sounds
                        .get(&settings.model)
                        .cloned()
                        .or_else(|| model.map(|value| value.sound_action))
                        .unwrap_or_else(|| settings.sound_whip.clone());
                    let catalog_dir = catalog.join("sounds");
                    let dir = if catalog_dir.join(format!("{stem}.wav")).is_file() {
                        catalog_dir
                    } else {
                        pack::default_pack_root().join("sounds")
                    };
                    let _ = std::fs::create_dir_all(&dir);
                    reveal_in_file_manager(&dir);
                }
                Ipc::Quit => {
                    quit_ipc.store(true, Ordering::Relaxed);
                }
                Ipc::Play(name) => {
                    crate::audio::play_wav(&crate::audio::sound_file(&name));
                }
                Ipc::Ignore => {}
            }
        })
        .build(&window)
        .map_err(|e| {
            launch_log(&format!("webview: {e}"));
            e.to_string()
        })?;
    launch_log("webview ok custom-protocol");

    event_loop.run(move |event, _, control_flow| {
        let _keep = &webview;
        *control_flow = ControlFlow::Wait;
        if let Ok(mut g) = pending.lock() {
            if let Some(js) = g.take() {
                let _ = webview.evaluate_script(&js);
            }
        }
        if quit.load(Ordering::Relaxed) {
            *control_flow = ControlFlow::Exit;
            return;
        }
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested | WindowEvent::Destroyed,
            ..
        } = event
        {
            quit.store(true, Ordering::Relaxed);
            *control_flow = ControlFlow::Exit;
        }
    });
}
