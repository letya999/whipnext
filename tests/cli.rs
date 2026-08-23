use std::path::PathBuf;

use whipnext::cli::{
    classify, current_settings_at, detect_json, help_text, launch_opts_from, parse_overlay_args,
    print_detect, proposed_settings, wants_settings_ui, Action, HELP,
};
use whipnext::detect::FakeProbe;

fn s(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

#[test]
fn status_code_prints_error() {
    assert_eq!(whipnext::cli::status_code(Ok(())), 0);
    assert_eq!(whipnext::cli::status_code(Err("boom".into())), 1);
    assert_eq!(whipnext::cli::run_from(vec!["--help".into()]), 0);
    assert_eq!(
        whipnext::cli::run_from(vec!["--overlay".into(), "--nope".into()]),
        1
    );
}

#[test]
fn help_text_lists_shipped_flags() {
    let t = help_text();
    assert_eq!(t, HELP);
    assert!(t.contains("whipnext"));
    assert!(t.contains("--detect"));
    assert!(t.contains("--demo"));
    assert!(t.contains("--dev"));
}

#[test]
fn classify_help_and_detect() {
    assert_eq!(classify(&s(&["--help"])).unwrap(), Action::Help);
    assert_eq!(classify(&s(&["-h"])).unwrap(), Action::Help);
    assert_eq!(classify(&s(&["--detect"])).unwrap(), Action::Detect);
    assert_eq!(classify(&s(&["detect"])).unwrap(), Action::Detect);
}

#[test]
fn classify_demo_is_overlay_not_settings() {
    let Action::Overlay(a) = classify(&s(&["--demo", "--always-show"])).unwrap() else {
        panic!("expected overlay");
    };
    assert!(a.demo);
    assert!(a.always_show);
    assert!(wants_settings_ui(&s(&[])));
    assert!(!wants_settings_ui(&s(&["--overlay"])));
}

#[test]
fn parse_overlay_args_all_flags_and_unknown() {
    let a = parse_overlay_args(&s(&[
        "--demo",
        "--overlay",
        "--auto-click",
        "--quit-after-strike",
        "--log",
        "x.log",
        "--dump-dir",
        "out",
        "--timeout-ms",
        "40",
        "--always-show",
        "--dev",
    ]))
    .unwrap();
    assert!(a.demo && a.auto_click && a.quit_after && a.always_show && a.dev);
    assert_eq!(a.timeout_ms, 40);
    assert_eq!(a.log_path.as_deref(), Some(std::path::Path::new("x.log")));
    assert_eq!(a.dump_dir.as_deref(), Some(std::path::Path::new("out")));
    let e = parse_overlay_args(&s(&["--nope"])).unwrap_err();
    assert!(e.contains("unknown arg --nope"));
    let z = parse_overlay_args(&s(&["--timeout-ms", "nope"])).unwrap();
    assert_eq!(z.timeout_ms, 0);
    let mock = parse_overlay_args(&s(&["--mock-inject", "--quit-after-punch"])).unwrap();
    assert!(mock.demo && mock.quit_after);
    let dev = parse_overlay_args(&s(&["--dev"])).unwrap();
    assert!(dev.dev);
    let opts = launch_opts_from(dev, whipnext::settings::Settings::default());
    assert!(opts.settings.dev_mode);
    assert!(opts
        .log_path
        .as_ref()
        .and_then(|p| p.file_name())
        .is_some_and(|n| n == "dev.log"));
}

#[test]
fn launch_opts_timeout_zero_is_none() {
    let mut a = parse_overlay_args(&s(&["--demo"])).unwrap();
    let opts = launch_opts_from(a.clone(), whipnext::settings::Settings::default());
    assert!(opts.timeout.is_none());
    a.timeout_ms = 15;
    let opts = launch_opts_from(a, whipnext::settings::Settings::default());
    assert_eq!(opts.timeout.unwrap().as_millis(), 15);
}

#[test]
fn detect_json_is_array_of_kinds_from_probe() {
    let mut probe = FakeProbe::default();
    probe.path_dirs = vec![PathBuf::from("/bin")];
    probe.files = vec![PathBuf::from("/bin/grok")];
    probe.running = vec![(9, "grok".into())];
    print_detect(&probe);
    let raw = detect_json(&probe);
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 6);
    for obj in arr {
        assert!(obj.get("kind").and_then(|k| k.as_str()).is_some());
        assert!(obj.get("installed").and_then(|k| k.as_bool()).is_some());
        assert!(obj.get("running").is_some());
        assert!(obj.get("pids").unwrap().is_array());
    }
    let grok = arr.iter().find(|o| o["kind"] == "grok").unwrap();
    assert_eq!(grok["installed"], true);
    assert_eq!(grok["running"], true);
    assert_eq!(grok["pids"][0], 9);
}

#[test]
fn current_settings_at_writes_proposed_when_missing() {
    let home = std::env::temp_dir().join(format!("whipnext-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let mut probe = FakeProbe::default();
    probe.path_dirs = vec![PathBuf::from("/bin")];
    probe.files = vec![PathBuf::from("/bin/claude")];
    probe.running = vec![(1, "cursor.exe".into()), (2, "notepad.exe".into())];
    let s = current_settings_at(&home, &probe);
    assert!(s.apps.contains_key("cursor"));
    assert!(s.apps.contains_key("claude"));
    assert!(home.join(".whipnext").join("settings.json").is_file());
    let again = proposed_settings(&probe);
    assert!(again.apps.contains_key("claude"));
    let _ = std::fs::remove_dir_all(&home);
}
