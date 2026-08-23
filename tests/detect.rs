use std::path::PathBuf;

use whipnext::detect::{catalog, FakeProbe};
use whipnext::kind::Kind;

#[test]
fn six_kinds_installed_vs_running_and_missing_not_installed() {
    let mut probe = FakeProbe::default();
    probe.home = PathBuf::from("/home/u");
    probe.local_app_data = Some(PathBuf::from("/la"));
    probe.path_dirs = vec![PathBuf::from("/bin")];
    probe.files = vec![
        PathBuf::from("/bin/grok"),
        PathBuf::from("/bin/claude"),
        PathBuf::from("/bin/codex"),
        PathBuf::from("/la/agy/bin/agy"),
        PathBuf::from("/bin/opencode"),
        // pi deliberately missing
    ];
    probe.running = vec![
        (111, "grok.exe".into()),
        (222, "claude".into()),
        (333, "node.exe".into()),
        (444, "codex".into()),
        (555, "opencode".into()),
    ];
    let cat = catalog(&probe);
    assert_eq!(cat.len(), 6);

    let by = |k: Kind| cat.iter().find(|s| s.kind == k).unwrap();
    assert!(by(Kind::Grok).installed && by(Kind::Grok).running);
    assert!(by(Kind::Claude).installed && by(Kind::Claude).running);
    assert!(by(Kind::Codex).installed && by(Kind::Codex).running);
    assert!(by(Kind::Antigravity).installed);
    assert!(!by(Kind::Antigravity).running);
    assert!(by(Kind::OpenCode).installed && by(Kind::OpenCode).running);
    assert!(!by(Kind::Pi).installed);
    assert!(!by(Kind::Pi).running);
    assert!(by(Kind::Pi).pids.is_empty());
    assert_eq!(by(Kind::Grok).pids, vec![111]);
}
