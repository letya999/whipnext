use whipnext::detect::{catalog, FakeProbe};
use whipnext::focus::{is_self, match_foreground, Foreground, Proc};
use whipnext::kind::Kind;
use whipnext::settings::{self, AppPref, Settings};

fn fg(exe: &str, title: &str, pid: u32, children: &[(u32, &str, u32)]) -> Foreground {
    let mut procs = vec![Proc {
        pid,
        name: exe.into(),
        parent: 0,
    }];
    for (cpid, n, parent) in children {
        procs.push(Proc {
            pid: *cpid,
            name: (*n).into(),
            parent: *parent,
        });
    }
    Foreground {
        pid,
        exe: exe.into(),
        title: title.into(),
        procs,
    }
}

fn settings_with(ids: &[&str], enabled: bool) -> Settings {
    let mut s = Settings::default();
    s.only_when_matched = true;
    for id in ids {
        s.apps.insert(
            (*id).into(),
            AppPref {
                enabled,
                ..Default::default()
            },
        );
    }
    s
}

#[test]
fn grok_cli_shows_civ6_hides() {
    let s = settings_with(&["grok"], true);
    let grok = fg("grok.exe", "Grok Build", 11, &[]);
    assert_eq!(match_foreground(&grok, &s).as_deref(), Some("grok"));
    let civ = fg("CivilizationVI.exe", "Sid Meier's Civilization VI", 12, &[]);
    assert_eq!(match_foreground(&civ, &s), None);
    let hidden = settings_with(&["grok"], false);
    assert_eq!(match_foreground(&grok, &hidden), None);
}

#[test]
fn windows_terminal_child_grok_shows_unless_disabled() {
    let mut s = settings_with(&["grok"], true);
    let wt = fg("WindowsTerminal.exe", "grok", 10, &[(22, "grok.exe", 10)]);
    assert_eq!(match_foreground(&wt, &s).as_deref(), Some("grok"));
    s.apps.get_mut("grok").unwrap().enabled = false;
    assert_eq!(match_foreground(&wt, &s), None);
}

#[test]
fn host_with_multiple_harness_children_does_not_guess() {
    let s = settings_with(&["grok", "opencode"], true);
    let host = fg(
        "WindowsTerminal.exe",
        "terminal",
        10,
        &[(22, "grok.exe", 10), (23, "opencode.exe", 10)],
    );
    assert_eq!(match_foreground(&host, &s), None);
}

#[test]
fn chatgpt_desktop_is_not_codex_cli() {
    let s = settings_with(&["codex"], true);
    let window = fg("ChatGPT.exe", "ChatGPT", 10, &[]);
    assert_eq!(match_foreground(&window, &s), None);
}

#[test]
fn terminal_shell_tree_identifies_cli_without_tab_title() {
    let s = settings_with(&["antigravity", "codex", "opencode"], true);
    let antigravity = fg(
        "powershell.exe",
        r"C:\Windows\system32\cmd.exe",
        10,
        &[(22, "agy.exe", 10)],
    );
    assert_eq!(
        match_foreground(&antigravity, &s).as_deref(),
        Some("antigravity")
    );
    let codex = fg("powershell.exe", "whatever", 11, &[(23, "codex.exe", 11)]);
    assert_eq!(match_foreground(&codex, &s).as_deref(), Some("codex"));
    let opencode = fg(
        "powershell.exe",
        "whatever",
        12,
        &[(24, "opencode.exe", 12)],
    );
    assert_eq!(match_foreground(&opencode, &s).as_deref(), Some("opencode"));
}

#[test]
fn terminal_title_without_agent_process_does_not_guess() {
    let s = settings_with(&["opencode"], true);
    let terminal = fg("WindowsTerminal.exe", "OC | Quick start command", 10, &[]);
    assert_eq!(match_foreground(&terminal, &s), None);
}

#[test]
fn propose_only_installed_ai_never_games() {
    let mut probe = FakeProbe::default();
    probe.files.push(std::path::PathBuf::from("/bin/grok.exe"));
    probe.path_dirs.push(std::path::PathBuf::from("/bin"));
    probe.running = vec![(1, "CivilizationVI.exe".into()), (2, "grok.exe".into())];
    let cat = catalog(&probe);
    let s = settings::propose_from_detect(&cat, &["civ6".into()]);
    assert!(s.apps.contains_key("grok"));
    assert!(!s.apps.contains_key("civ6"));
    assert_eq!(
        cat.iter().find(|a| a.kind == Kind::Grok).map(|a| a.running),
        Some(true)
    );
}

#[test]
fn whipnext_settings_is_self() {
    assert!(is_self("whipnext.exe"));
    assert!(!is_self("grok.exe"));
    assert!(whipnext::focus::is_herdr_exe("herdr.exe"));
    assert!(!whipnext::focus::is_herdr_exe("grok.exe"));
}

#[test]
fn disabling_grok_hides_on_grok_foreground() {
    let mut s = settings_with(&["grok"], false);
    let g = fg(
        "WindowsTerminal.exe",
        "grok — whipnext",
        10,
        &[(22, "grok.exe", 10)],
    );
    assert_eq!(match_foreground(&g, &s), None);
    s.apps.get_mut("grok").unwrap().enabled = true;
    assert_eq!(match_foreground(&g, &s).as_deref(), Some("grok"));
}
