use whipnext::inject::{execute_live, live_attempts, LiveAttempt, NEXT_PROMPT};
use whipnext::kind::Kind;
use whipnext::session::Target;

fn target(pid: Option<u32>, pane: Option<&str>, name: Option<&str>) -> Target {
    Target {
        kind: Kind::Grok,
        session_id: None,
        pid,
        pane_id: pane.map(|s| s.into()),
        herdr_name: name.map(|s| s.into()),
    }
}

#[test]
fn live_attempts_herdr_pane_then_pid() {
    let a = live_attempts(&target(Some(7), Some("wG:p4"), Some("worker")));
    match &a[..] {
        [LiveAttempt::Herdr { pane }, LiveAttempt::Pid(7)] if pane == "wG:p4" => {}
        other => panic!("{other:?}"),
    }
    let a = live_attempts(&target(None, None, Some("worker-1")));
    match &a[..] {
        [LiveAttempt::Herdr { pane }] if pane == "worker-1" => {}
        other => panic!("{other:?}"),
    }
    assert!(live_attempts(&target(None, None, None)).is_empty());
}

#[test]
fn execute_live_herdr_success_skips_pid() {
    let mut pids = Vec::new();
    let mut herdr = Vec::new();
    let mut pid_fn = |pid: u32, text: &str| {
        pids.push((pid, text.to_string()));
        assert_eq!(text, NEXT_PROMPT);
        assert_ne!(text, "/next");
        Ok(())
    };
    let mut herdr_fn = |bin: &str, args: &[String]| {
        herdr.push((bin.to_string(), args.to_vec()));
        Ok(())
    };
    execute_live(
        &target(Some(22), Some("wG:p4"), None),
        &mut pid_fn,
        &mut herdr_fn,
    )
    .unwrap();
    assert!(pids.is_empty());
    assert_eq!(herdr[0].1, vec!["agent", "prompt", "wG:p4", "next"]);
}

#[test]
fn execute_live_herdr_fail_falls_back_to_pid() {
    let mut pids = Vec::new();
    let mut pid_fn = |pid: u32, text: &str| {
        pids.push((pid, text.to_string()));
        Ok(())
    };
    let mut herdr_fn = |_: &str, _: &[String]| Err("no pane".into());
    execute_live(
        &target(Some(22), Some("wG:p4"), None),
        &mut pid_fn,
        &mut herdr_fn,
    )
    .unwrap();
    assert_eq!(pids, vec![(22, "next".into())]);
}

#[test]
fn execute_live_both_fail_and_missing() {
    let mut pid_fn = |_: u32, _: &str| Err("pid-miss".into());
    let mut herdr_fn = |_: &str, _: &[String]| Err("spawn".into());
    let err = execute_live(
        &target(Some(1), Some("p"), None),
        &mut pid_fn,
        &mut herdr_fn,
    )
    .unwrap_err();
    assert_eq!(err, "pid-miss");
    let mut ok = |_: u32, _: &str| Ok(());
    let mut ok2 = |_: &str, _: &[String]| Ok(());
    let err = execute_live(&target(None, None, None), &mut ok, &mut ok2).unwrap_err();
    assert_eq!(err, "no pane_id or pid on target");
}

#[test]
fn live_inject_os_missing_target_errors_without_sendinput() {
    let err = whipnext::inject_os::live_inject(&target(None, None, None)).unwrap_err();
    assert_eq!(err, "no pane_id or pid on target");
}

#[test]
fn live_inject_os_dead_pid_without_pane_errors() {
    let err = whipnext::inject_os::live_inject(&target(Some(u32::MAX), None, None)).unwrap_err();
    assert!(err.contains("no window") || err.contains("pid"), "{err}");
}

#[test]
fn live_inject_os_herdr_pane_attempts_spawn() {
    let r = whipnext::inject_os::live_inject(&target(None, Some("whipnext-no-pane"), None));
    match r {
        Ok(()) => {}
        Err(e) => assert!(!e.is_empty(), "{e}"),
    }
}

#[test]
fn spawn_herdr_missing_bin_errors() {
    let err =
        whipnext::inject_os::spawn_herdr("definitely-missing-whipnext-bin-xyz", &["--help".into()])
            .unwrap_err();
    assert!(!err.is_empty(), "{err}");
    let _ = whipnext::inject_os::command_hidden("definitely-missing-whipnext-bin-xyz");
}
