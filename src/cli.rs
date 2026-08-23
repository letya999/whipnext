//! Binary entry: flag parse, `--detect` JSON, overlay vs settings UI.

use std::path::PathBuf;
use std::time::Duration;

use crate::detect::{self, Probe};
use crate::focus::surface_for_exe;
use crate::kind::Kind;
use crate::os_probe::SystemProbe;
use crate::overlay::{self, LaunchOpts};
use crate::settings;

pub const HELP: &str = "whipnext                 # settings window + overlay\n\
whipnext --detect        # JSON of installed/running agents\n\
whipnext --demo          # overlay only, no inject\n\
whipnext --dev           # inject traces in ~/.whipnext/dev.log";

pub fn help_text() -> &'static str {
    HELP
}

pub fn detect_json(probe: &dyn Probe) -> String {
    let cat = detect::catalog(probe);
    let v: Vec<_> = cat
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "kind": s.kind.as_str(),
                "installed": s.installed,
                "running": s.running,
                "pids": s.pids,
            })
        })
        .collect();
    let _ = Kind::ALL;
    serde_json::to_string_pretty(&v).unwrap()
}

pub fn print_detect(probe: &dyn Probe) {
    println!("{}", detect_json(probe));
}

pub fn proposed_settings(probe: &dyn Probe) -> settings::Settings {
    let cat = detect::catalog(probe);
    let extra: Vec<String> = probe
        .running()
        .into_iter()
        .filter_map(|(_, n)| surface_for_exe(&n).map(|s| s.to_string()))
        .filter(|id| settings::is_ai_ide(id))
        .collect();
    settings::propose_from_detect(&cat, &extra)
}

pub fn current_settings_at(home: &std::path::Path, probe: &dyn Probe) -> settings::Settings {
    settings::load_or_propose(&settings::settings_path(home), proposed_settings(probe))
}

pub fn current_settings(probe: &dyn Probe) -> settings::Settings {
    current_settings_at(&settings::home_dir(), probe)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverlayArgs {
    pub demo: bool,
    pub auto_click: bool,
    pub quit_after: bool,
    pub log_path: Option<PathBuf>,
    pub dump_dir: Option<PathBuf>,
    pub timeout_ms: u64,
    pub always_show: bool,
    pub dev: bool,
}

pub fn parse_overlay_args(argv: &[String]) -> Result<OverlayArgs, String> {
    let mut out = OverlayArgs::default();
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--demo" | "--mock-inject" => out.demo = true,
            "--overlay" => {}
            "--auto-click" => out.auto_click = true,
            "--quit-after-strike" | "--quit-after-punch" => out.quit_after = true,
            "--log" => {
                i += 1;
                out.log_path = argv.get(i).map(PathBuf::from);
            }
            "--dump-dir" => {
                i += 1;
                out.dump_dir = argv.get(i).map(PathBuf::from);
            }
            "--timeout-ms" => {
                i += 1;
                out.timeout_ms = argv.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
            "--always-show" => out.always_show = true,
            "--dev" => out.dev = true,
            other => return Err(format!("unknown arg {other}")),
        }
        i += 1;
    }
    Ok(out)
}

pub fn launch_opts_from(args: OverlayArgs, mut settings: settings::Settings) -> LaunchOpts {
    if args.dev {
        settings.dev_mode = true;
    }
    let log_path = args.log_path.or_else(|| {
        settings
            .dev_mode
            .then(|| settings::dev_log_path(&settings::home_dir()))
    });
    LaunchOpts {
        demo: args.demo,
        auto_click: args.auto_click,
        quit_after: args.quit_after,
        log_path,
        timeout: if args.timeout_ms > 0 {
            Some(Duration::from_millis(args.timeout_ms))
        } else {
            None
        },
        dump_dir: args.dump_dir,
        force_target: None,
        always_show: args.always_show,
        settings,
        live: None,
        quit: None,
    }
}

pub fn wants_settings_ui(args: &[String]) -> bool {
    !args.iter().any(|a| a == "--demo" || a == "--overlay")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Help,
    Detect,
    Overlay(OverlayArgs),
    SettingsUi,
}

pub fn classify(args: &[String]) -> Result<Action, String> {
    match args.first().map(|s| s.as_str()) {
        Some("--detect") | Some("detect") => Ok(Action::Detect),
        Some("--help") | Some("-h") => Ok(Action::Help),
        _ if wants_settings_ui(args) => Ok(Action::SettingsUi),
        _ => Ok(Action::Overlay(parse_overlay_args(args)?)),
    }
}

pub fn status_code(result: Result<(), String>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

pub fn run_from(args: Vec<String>) -> i32 {
    status_code(dispatch(&args))
}

pub fn dispatch(args: &[String]) -> Result<(), String> {
    match classify(args)? {
        Action::Detect => {
            print_detect(&SystemProbe);
            let _ = SystemProbe.path_dirs();
            Ok(())
        }
        Action::Help => {
            eprint!("{HELP}");
            Ok(())
        }
        Action::Overlay(parsed) => {
            overlay::run_overlay(launch_opts_from(parsed, current_settings(&SystemProbe)))
        }
        Action::SettingsUi => run_overlay_only(args),
    }
}

pub fn run_overlay_only(argv: &[String]) -> Result<(), String> {
    let parsed = parse_overlay_args(argv)?;
    overlay::run_overlay(launch_opts_from(parsed, current_settings(&SystemProbe)))
}
