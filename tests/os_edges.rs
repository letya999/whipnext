use std::path::PathBuf;

use whipnext::audio::{assets_dir, next_wav, play_wav, sound_file, whip_wav, OsAudio};
use whipnext::os_probe::{cursor_pos, foreground_pid, foreground_snapshot, SystemProbe};
use whipnext::whip::Audio;

#[test]
fn system_probe_and_foreground_are_callable() {
    use whipnext::detect::Probe;
    let p = SystemProbe;
    let dirs = p.path_dirs();
    assert!(!dirs.is_empty() || std::env::var_os("PATH").is_none());
    let home = p.home();
    assert!(home.as_os_str().len() > 0);
    let _ = p.local_app_data();
    let running = p.running();
    assert!(
        running.iter().any(|(pid, _)| *pid > 0),
        "expected at least one live process, got {running:?}"
    );
    assert!(!p.exists(std::path::Path::new("definitely-missing-whipnext-xyz")));
    let _ = foreground_pid();
    let _ = foreground_snapshot();
    let _ = cursor_pos();
}

#[test]
fn audio_paths_and_async_play() {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::env::set_var("WHIPNEXT_ASSETS", &assets);
    assert_eq!(assets_dir(), assets);
    assert!(whip_wav().is_file());
    assert!(next_wav().is_file());
    assert!(sound_file("whip").is_file());
    let missing = sound_file("definitely-not-a-stem");
    assert!(missing.ends_with("definitely-not-a-stem.wav"));
    play_wav(&whip_wav());
    play_wav(std::path::Path::new("nope.wav"));
    let mut a = OsAudio::default();
    a.play_whip();
    a.play_next();
    std::env::remove_var("WHIPNEXT_ASSETS");

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    assert_eq!(
        whipnext::audio::assets_dir_from(Some("D:/pack-root".into()), None, manifest.clone()),
        PathBuf::from("D:/pack-root")
    );
    assert_eq!(
        whipnext::audio::assets_dir_from(
            None,
            Some(PathBuf::from("C:/definitely-missing-whipnext-xyz/nope.exe")),
            manifest.clone()
        ),
        manifest
    );
    assert_eq!(
        whipnext::audio::assets_dir_from(None, Some(PathBuf::from("")), manifest.clone()),
        manifest
    );
    let _ = assets_dir();
    let beside_exe = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("dummy.exe");
    let beside =
        whipnext::audio::assets_dir_from(None, Some(beside_exe), PathBuf::from("fallback"));
    assert_eq!(beside, PathBuf::from("fallback"));
    let fake_root =
        std::env::temp_dir().join(format!("whipnext-assets-beside-{}", std::process::id()));
    let _ = std::fs::create_dir_all(fake_root.join("assets/pack"));
    let got = whipnext::audio::assets_dir_from(
        None,
        Some(fake_root.join("whipnext.exe")),
        PathBuf::from("fallback"),
    );
    assert_eq!(got, fake_root.join("assets"));
    let frames_root =
        std::env::temp_dir().join(format!("whipnext-assets-frames-{}", std::process::id()));
    let _ = std::fs::create_dir_all(frames_root.join("assets/frames"));
    let got = whipnext::audio::assets_dir_from(
        None,
        Some(frames_root.join("whipnext.exe")),
        PathBuf::from("fallback"),
    );
    assert_eq!(got, frames_root.join("assets"));
    let _ = std::fs::remove_dir_all(&fake_root);
    let _ = std::fs::remove_dir_all(&frames_root);
}

#[test]
fn os_home_prefers_userprofile_then_home_then_dot() {
    use std::ffi::OsString;
    assert_eq!(
        whipnext::os_probe::os_home(Some(OsString::from("U")), Some(OsString::from("H"))),
        PathBuf::from("U")
    );
    assert_eq!(
        whipnext::os_probe::os_home(None, Some(OsString::from("H"))),
        PathBuf::from("H")
    );
    assert_eq!(whipnext::os_probe::os_home(None, None), PathBuf::from("."));
}

#[cfg(windows)]
#[test]
fn inject_focus_skips_restore_when_not_minimized() {
    let src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/inject_os_win_tests.rs"),
    )
    .unwrap();
    assert!(src.contains("focus_needs_restore"));
    assert!(src.contains("IsIconic"));
    assert!(src.contains("GetAsyncKeyState"));
    assert!(src.contains("BlockInput(1)"));
    assert!(src.contains("BlockInput(0)"));
    assert!(src.contains("impl Drop for InputBlock"));
    assert!(src.contains("release_held_modifiers"));
    assert!(src.contains("INJECT_SETTLE_MS"));
    assert!(src.contains("VkKeyScanExW"));
    assert!(src.contains("SetClipboardData"));
    assert!(src.contains("VK_CONTROL"));
    assert!(src.contains("VK_V"));
    assert!(src.contains("TypeMethod::Paste"));
    assert!(src.contains("SendInput failed"));
    assert!(src.contains("fn send_submit()"));
    assert!(src.contains("key_input(VK_RETURN"));

    let overlay_src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/overlay_win_tests.rs"),
    )
    .unwrap();
    assert!(!overlay_src.contains("GetAsyncKeyState(VK_CONTROL)"));
}

#[cfg(windows)]
#[test]
fn layered_window_roundtrip() {
    use whipnext::overlay::premultiply_bgra;
    use whipnext::overlay_win::Layered;
    let mut layer = Layered::new(16, 16).expect("CreateWindowExW");
    layer.set_pos(0, 0);
    layer.set_visible(false);
    layer.pump();
    let rgba = vec![0u8; 16 * 16 * 4];
    layer.blit_rgba(&rgba);
    let short = vec![1u8, 2, 3];
    layer.blit_rgba(&short);
    let _ = layer.clicked_opaque(&rgba, 0, 0);
    let _ = layer.escape_over_window(0, 0);
    layer.set_visible(true);
    let out = premultiply_bgra(&[10, 20, 30, 128]);
    assert_eq!(out.len(), 4);
    assert_eq!(out[3], 128);
}

#[test]
fn resolve_live_target_does_not_panic() {
    let _ = whipnext::live::resolve_live_target();
}

#[test]
fn cli_dispatch_help_and_detect() {
    whipnext::cli::dispatch(&["--help".into()]).unwrap();
    whipnext::cli::dispatch(&["--detect".into()]).unwrap();
    let home = std::env::temp_dir().join(format!("whipnext-cli-home-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&home);
    std::env::set_var("WHIPNEXT_HOME", &home);
    let _ = whipnext::cli::current_settings(&whipnext::os_probe::SystemProbe);
    std::env::remove_var("WHIPNEXT_HOME");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn classify_unknown_overlay_arg_errors() {
    let err = whipnext::cli::classify(&["--overlay".into(), "--bogus".into()]).unwrap_err();
    assert!(err.contains("unknown arg --bogus"));
}

#[cfg_attr(target_os = "macos", ignore = "AppKit requires the main thread")]
#[test]
fn run_overlay_only_demo_times_out() {
    let home = std::env::temp_dir().join(format!("whipnext-ovl-home-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&home);
    std::env::set_var("WHIPNEXT_HOME", &home);
    std::env::set_var(
        "WHIPNEXT_ASSETS",
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
    );
    whipnext::cli::run_overlay_only(&[
        "--demo".into(),
        "--always-show".into(),
        "--timeout-ms".into(),
        "80".into(),
    ])
    .unwrap();
    whipnext::cli::dispatch(&[
        "--demo".into(),
        "--always-show".into(),
        "--timeout-ms".into(),
        "1".into(),
    ])
    .unwrap();
    std::env::remove_var("WHIPNEXT_ASSETS");
    std::env::remove_var("WHIPNEXT_HOME");
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg_attr(target_os = "macos", ignore = "AppKit requires the main thread")]
#[test]
fn dispatch_timeout_without_demo_still_runs_overlay() {
    let home = std::env::temp_dir().join(format!("whipnext-ui-home-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&home);
    std::env::set_var("WHIPNEXT_HOME", &home);
    std::env::set_var(
        "WHIPNEXT_ASSETS",
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
    );
    whipnext::cli::dispatch(&["--timeout-ms".into(), "80".into(), "--always-show".into()]).unwrap();
    std::env::remove_var("WHIPNEXT_ASSETS");
    std::env::remove_var("WHIPNEXT_HOME");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn classify_empty_is_settings_ui() {
    assert_eq!(
        whipnext::cli::classify(&[]).unwrap(),
        whipnext::cli::Action::SettingsUi
    );
}
