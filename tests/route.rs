use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use whipnext::detect::{catalog, FakeProbe};
use whipnext::herdr::{parse_agent_list, prompt_next_args};
use whipnext::inject::{CommandInjector, Injector, NEXT_PROMPT};
use whipnext::kind::Kind;
use whipnext::session::{select_current, GrokSession, Target};

fn fixture() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/herdr_agents.json"),
    )
    .unwrap()
}

#[test]
fn focused_herdr_pane_gets_next_not_the_other_live_session() {
    let agents = parse_agent_list(&fixture()).unwrap();
    assert_eq!(agents.len(), 3);
    let t = select_current(Some(&agents), &[], None, &[], &[], &[]).expect("focused pane");
    assert_eq!(t.kind, Kind::Grok);
    assert_eq!(t.pane_id.as_deref(), Some("wG:p4"));
    assert_eq!(t.session_id.as_deref(), Some("front-grok"));
    assert_ne!(t.pane_id.as_deref(), Some("wG:p2"));
    assert_ne!(t.pane_id.as_deref(), Some("wG:p9"));
    let args = prompt_next_args(t.pane_id.as_deref().unwrap());
    assert_eq!(args, vec!["agent", "prompt", "wG:p4", "next"]);
    assert_eq!(args[3], NEXT_PROMPT);
}

#[test]
fn command_injector_calls_herdr_agent_prompt_pane_next() {
    let got = Rc::new(RefCell::new(Vec::new()));
    let g = got.clone();
    let mut inj = CommandInjector {
        herdr_bin: "herdr".into(),
        run: Box::new(move |bin, args| {
            g.borrow_mut().push((bin.to_string(), args.to_vec()));
        }),
    };
    inj.submit(whipnext::inject::InjectRequest {
        target: Target {
            kind: Kind::Grok,
            session_id: Some("front-grok".into()),
            pid: None,
            pane_id: Some("wG:p4".into()),
            herdr_name: Some("worker-1".into()),
        },
        payload: "next".into(),
        submit: true,
    });
    let rec = got.borrow();
    assert_eq!(rec.len(), 1);
    assert_eq!(rec[0].0, "herdr");
    assert_eq!(rec[0].1, vec!["agent", "prompt", "wG:p4", "next"]);
}

#[test]
fn command_injector_uses_request_phrase() {
    let got = Rc::new(RefCell::new(Vec::new()));
    let g = got.clone();
    let mut inj = CommandInjector {
        herdr_bin: "herdr".into(),
        run: Box::new(move |bin, args| {
            g.borrow_mut().push((bin.to_string(), args.to_vec()));
        }),
    };
    inj.submit(whipnext::inject::InjectRequest {
        target: Target {
            kind: Kind::Grok,
            session_id: None,
            pid: None,
            pane_id: Some("wG:p4".into()),
            herdr_name: None,
        },
        payload: "keep going".into(),
        submit: true,
    });
    assert_eq!(got.borrow()[0].1[3], "keep going");
}

#[test]
fn command_injector_without_pane_or_name_is_noop() {
    let got = Rc::new(RefCell::new(0usize));
    let g = got.clone();
    let mut inj = CommandInjector {
        herdr_bin: "herdr".into(),
        run: Box::new(move |_bin, _args| {
            *g.borrow_mut() += 1;
        }),
    };
    inj.submit(whipnext::inject::InjectRequest {
        target: Target {
            kind: Kind::Grok,
            session_id: None,
            pid: Some(1),
            pane_id: None,
            herdr_name: None,
        },
        payload: "next".into(),
        submit: true,
    });
    assert_eq!(*got.borrow(), 0);
}

#[test]
fn os_foreground_picks_matching_pid_not_other_live_agent() {
    let mut probe = FakeProbe::default();
    probe.running = vec![(222, "grok.exe".into()), (333, "claude.exe".into())];
    let cat = catalog(&probe);
    let grok = GrokSession {
        session_id: "front-grok".into(),
        pid: 222,
        cwd: "C:\\proj-a".into(),
    };
    let other = GrokSession {
        session_id: "other".into(),
        pid: 999,
        cwd: "C:\\proj-b".into(),
    };
    let chosen = select_current(None, &cat, Some(222), &[], &[grok, other], &[222]).unwrap();
    assert_eq!(chosen.kind, Kind::Grok);
    assert_eq!(chosen.pid, Some(222));
    assert_ne!(chosen.pid, Some(333));
}
