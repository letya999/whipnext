use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_whipnext"))
}

#[test]
fn binary_help_prints_flags() {
    let out = bin().arg("--help").output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("whipnext"), "{text}");
    assert!(text.contains("--detect"), "{text}");
    assert!(text.contains("--demo"), "{text}");
    assert!(text.contains("--dev"), "{text}");
}

#[test]
fn binary_unknown_arg_exits_nonzero() {
    let out = bin()
        .args(["--overlay", "--definitely-not-a-flag"])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("unknown arg") || !out.status.success(),
        "status={} stderr={err}",
        out.status
    );
}

#[test]
fn binary_detect_prints_json_kinds() {
    let out = bin().arg("--detect").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect(&stdout);
    let arr = v.as_array().expect("array");
    assert!(!arr.is_empty());
    for obj in arr {
        assert!(obj.get("kind").and_then(|k| k.as_str()).is_some());
        assert!(obj.get("installed").is_some());
    }
}
