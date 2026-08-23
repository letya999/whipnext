use std::path::PathBuf;

use whipnext::detect::{catalog, FakeProbe};
use whipnext::focus::{
    is_host, match_foreground, overlay_surface, related_pids, surface_for_exe, surface_from_title,
    Foreground, Proc,
};
use whipnext::herdr::{focused_agent, parse_agent_list, prompt_next_args, HERDR_BIN};
use whipnext::kind::{extra_bin_dirs, Kind};
use whipnext::live::{
    grok_pids, grok_sessions_from_json, parse_herdr_output, resolve, resolve_from_probe,
};
use whipnext::session::{
    parse_active_sessions, retarget_for_foreground, select_current, GrokSession, Target,
};
use whipnext::settings::{AppPref, Settings};

fn fixture() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/herdr_agents.json"),
    )
    .unwrap()
}

fn st(kind: Kind, pids: Vec<u32>) -> whipnext::detect::AgentStatus {
    whipnext::detect::AgentStatus {
        kind,
        installed: true,
        install_path: None,
        running: !pids.is_empty(),
        pids,
    }
}

#[test]
fn herdr_skips_unknown_and_none_focused_falls_through() {
    let nameless = r#"{"result":{"agents":[{"agent":"grok","pane_id":"p0","focused":true}]}}"#;
    let t = whipnext::session::select_current(
        Some(&parse_agent_list(nameless).unwrap()),
        &[],
        None,
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert!(t.herdr_name.is_none());
    let json = r#"{"result":{"agents":[{"agent":"unknown","pane_id":"x","focused":true},{"agent":"pi","pane_id":"p1","focused":false}]}}"#;
    let agents = parse_agent_list(json).unwrap();
    assert_eq!(agents.len(), 1);
    assert!(focused_agent(&agents).is_none());
    assert!(parse_agent_list("nope").is_err());
    assert!(parse_herdr_output(false, &fixture()).is_none());
    assert!(parse_herdr_output(true, "nope").is_none());
    assert_eq!(prompt_next_args("p")[0], "agent");
    assert_eq!(HERDR_BIN, "herdr");
}

#[test]
fn three_grok_pids_inject_the_focused_window_not_herdr() {
    let agents = parse_agent_list(&fixture()).unwrap();
    let cat = vec![st(Kind::Grok, vec![111, 222, 333])];
    let t = select_current(Some(&agents), &cat, Some(333), &[], &[], &[]).unwrap();
    assert_eq!(t.pid, Some(333));
    assert!(t.pane_id.is_none());
    let t = select_current(Some(&agents), &cat, Some(10), &[222], &[], &[]).unwrap();
    assert_eq!(t.pid, Some(222));
}

#[test]
fn select_current_prefers_exact_pid_among_two_matches() {
    let cat = vec![st(Kind::Grok, vec![10]), st(Kind::Claude, vec![11])];
    let t = select_current(None, &cat, Some(10), &[11], &[], &[]).unwrap();
    assert_eq!(t.kind, Kind::Grok);
    assert_eq!(t.pid, Some(10));
    let only_anc = select_current(None, &cat, Some(99), &[11], &[], &[]).unwrap();
    assert_eq!(only_anc.pid, Some(11));
}

#[test]
fn grok_session_matches_foreground_pid() {
    let grok = GrokSession {
        session_id: "s1".into(),
        pid: 50,
        cwd: "/x".into(),
    };
    let t = select_current(None, &[], Some(50), &[], &[grok], &[50]).unwrap();
    assert_eq!(t.session_id.as_deref(), Some("s1"));
    assert!(select_current(None, &[], Some(1), &[], &[], &[]).is_none());
    assert!(parse_active_sessions("[]").unwrap().is_empty());
    assert!(parse_active_sessions("{").is_err());
    let rows = parse_active_sessions(r#"[{"session_id":"a","pid":1,"cwd":"c"}]"#).unwrap();
    assert_eq!(rows[0].session_id, "a");
    assert!(grok_sessions_from_json(Some("{")).is_empty());
}

#[test]
fn resolve_from_probe_uses_focused_herdr() {
    let mut probe = FakeProbe::default();
    probe.running = vec![(222, "grok.exe".into())];
    let t = resolve_from_probe(&probe, true, &fixture(), None, Some(222)).unwrap();
    assert_eq!(t.pid, Some(222), "focused grok window wins over Herdr pane");
    assert!(t.pane_id.is_none());
    let herdr_only = resolve_from_probe(&probe, true, &fixture(), None, None).unwrap();
    assert_eq!(herdr_only.pane_id.as_deref(), Some("wG:p4"));
    let cat = catalog(&probe);
    assert_eq!(grok_pids(&cat), vec![222]);
    assert!(grok_pids(&[]).is_empty());
    assert!(resolve(None, &[], None, &[]).is_none());
}

#[test]
fn retarget_clears_pane_unless_self() {
    let mut s = Settings::default();
    s.apps.insert("grok".into(), AppPref::default());
    let t = Target {
        kind: Kind::Grok,
        session_id: None,
        pid: Some(1),
        pane_id: Some("wG:p4".into()),
        herdr_name: None,
    };
    let fg = Foreground {
        pid: 99,
        exe: "grok.exe".into(),
        title: "Grok".into(),
        procs: vec![],
    };
    let out = retarget_for_foreground(t.clone(), Some(&fg), &s);
    assert_eq!(out.pid, Some(99));
    assert!(out.pane_id.is_none());
    let self_fg = Foreground {
        pid: 5,
        exe: "whipnext.exe".into(),
        title: "".into(),
        procs: vec![],
    };
    let out = retarget_for_foreground(t.clone(), Some(&self_fg), &s);
    assert_eq!(out.pid, Some(1));
    let herdr_fg = Foreground {
        pid: 77,
        exe: "herdr.exe".into(),
        title: "herdr".into(),
        procs: vec![],
    };
    let out = retarget_for_foreground(t.clone(), Some(&herdr_fg), &s);
    assert!(out.pid.is_none());
    assert_eq!(out.pane_id.as_deref(), Some("wG:p4"));
    let wt = Foreground {
        pid: 40,
        exe: "WindowsTerminal.exe".into(),
        title: "codex".into(),
        procs: vec![],
    };
    let out = retarget_for_foreground(t.clone(), Some(&wt), &s);
    assert!(out.pid.is_none());
    assert_eq!(out.pane_id.as_deref(), Some("wG:p4"));
    let out = retarget_for_foreground(t, None, &s);
    assert_eq!(out.pid, Some(1));
}

#[test]
fn related_pids_include_host_and_agent_child() {
    let fg = Foreground {
        pid: 10,
        exe: "WindowsTerminal.exe".into(),
        title: "claude".into(),
        procs: vec![
            Proc {
                pid: 10,
                name: "WindowsTerminal.exe".into(),
                parent: 1,
            },
            Proc {
                pid: 1,
                name: "explorer.exe".into(),
                parent: 0,
            },
            Proc {
                pid: 22,
                name: "claude.exe".into(),
                parent: 10,
            },
        ],
    };
    let ids = related_pids(&fg);
    assert!(ids.contains(&10));
    assert!(ids.contains(&22));
    assert!(ids.contains(&1));
    let looped = Foreground {
        pid: 5,
        exe: "x.exe".into(),
        title: "".into(),
        procs: vec![Proc {
            pid: 5,
            name: "x.exe".into(),
            parent: 5,
        }],
    };
    assert_eq!(related_pids(&looped), vec![5]);
    let missing = Foreground {
        pid: 9,
        exe: "y.exe".into(),
        title: "".into(),
        procs: vec![],
    };
    assert_eq!(related_pids(&missing), vec![9]);
}

#[test]
fn kind_and_surfaces_cover_aliases() {
    assert_eq!(Kind::from_herdr("AGY"), Some(Kind::Antigravity));
    assert_eq!(Kind::from_herdr("nope"), None);
    assert_eq!(
        Kind::from_binary_name(r"C:\bin\codex.cmd"),
        Some(Kind::Codex)
    );
    assert_eq!(Kind::from_binary_name("codex.ps1"), Some(Kind::Codex));
    assert_eq!(Kind::from_binary_name("pi.EXE"), Some(Kind::Pi));
    for k in Kind::ALL {
        assert!(!k.herdr_kind().is_empty());
        assert!(!k.as_str().is_empty());
    }
    assert_eq!(Kind::Antigravity.herdr_kind(), "agy");
    assert_eq!(Kind::Grok.as_str(), "grok");
    assert!(Kind::Codex.binaries().iter().any(|b| *b == "codex.cmd"));
    let dirs = extra_bin_dirs(
        std::path::Path::new("/h"),
        Some(std::path::Path::new("/la")),
    );
    assert!(dirs
        .iter()
        .any(|p| p.ends_with("agy/bin") || p.ends_with("agy\\bin")));
    assert_eq!(extra_bin_dirs(std::path::Path::new("/h"), None).len(), 3);
    assert_eq!(surface_for_exe("Cursor.exe"), Some("cursor"));
    assert_eq!(surface_for_exe("zed"), Some("zed"));
    assert_eq!(surface_for_exe("notepad"), None);
    assert!(is_host("WindowsTerminal.exe"));
    assert_eq!(surface_from_title("Sid Meier's Civilization VI"), None);
    assert_eq!(surface_from_title("claude — project"), Some("claude"));
    assert_eq!(surface_from_title("Open Code — project"), Some("opencode"));
    assert_eq!(surface_from_title(" pi session"), Some("pi"));
}

#[test]
fn match_foreground_always_and_title_host() {
    let mut s = Settings::default();
    s.only_when_matched = false;
    let fg = Foreground {
        pid: 1,
        exe: "chrome.exe".into(),
        title: "x".into(),
        procs: vec![],
    };
    assert_eq!(match_foreground(&fg, &s).as_deref(), Some("always"));
    s.only_when_matched = true;
    s.apps.insert("codex".into(), AppPref::default());
    let wt = Foreground {
        pid: 10,
        exe: "WindowsTerminal.exe".into(),
        title: "codex".into(),
        procs: vec![Proc {
            pid: 10,
            name: "WindowsTerminal.exe".into(),
            parent: 0,
        }],
    };
    assert_eq!(match_foreground(&wt, &s), None);
    overlay_surface(false, Some(&wt), None, &s);
    assert_eq!(surface_from_title("hello notepad"), None);
    s.apps.get_mut("codex").unwrap().enabled = false;
    assert_eq!(match_foreground(&wt, &s), None);
    s.apps.get_mut("codex").unwrap().enabled = true;
    s.apps.insert(
        "grok".into(),
        AppPref {
            enabled: false,
            ..Default::default()
        },
    );
    let cyclic = Foreground {
        pid: 10,
        exe: "WindowsTerminal.exe".into(),
        title: "plain".into(),
        procs: vec![
            Proc {
                pid: 10,
                name: "WindowsTerminal.exe".into(),
                parent: 11,
            },
            Proc {
                pid: 11,
                name: "grok.exe".into(),
                parent: 10,
            },
        ],
    };
    assert_eq!(match_foreground(&cyclic, &s), None);
    s.apps.get_mut("grok").unwrap().enabled = true;
    assert_eq!(match_foreground(&cyclic, &s).as_deref(), Some("grok"));
    let self_always = Foreground {
        pid: 1,
        exe: "whipnext.exe".into(),
        title: "".into(),
        procs: vec![],
    };
    let mut open = Settings::default();
    open.only_when_matched = false;
    assert_eq!(
        overlay_surface(false, Some(&self_always), Some("always"), &open).as_deref(),
        Some("always")
    );
}

#[test]
fn command_injector_uses_herdr_name_when_no_pane() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use whipnext::inject::{CommandInjector, Injector};
    let got = Rc::new(RefCell::new(Vec::new()));
    let g = got.clone();
    let mut inj = CommandInjector {
        herdr_bin: "herdr".into(),
        run: Box::new(move |bin, args| g.borrow_mut().push((bin.to_string(), args.to_vec()))),
    };
    inj.submit(whipnext::inject::InjectRequest {
        target: Target {
            kind: Kind::Grok,
            session_id: None,
            pid: None,
            pane_id: None,
            herdr_name: Some("worker-1".into()),
        },
        payload: "next".into(),
        submit: true,
    });
    assert_eq!(got.borrow()[0].1[2], "worker-1");
}
