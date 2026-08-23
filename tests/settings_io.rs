use whipnext::settings::{
    home_dir, load, load_or_propose, merge_propose, save, settings_path, AppPref, Settings,
};

fn tmp(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("whipnext-set-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn load_rejects_bad_json_and_missing() {
    let home = tmp("bad");
    let path = settings_path(&home);
    assert!(load(&path).is_err());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{not json").unwrap();
    assert!(load(&path).unwrap_err().contains("settings:"));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn load_or_propose_repairs_bad_file_and_merges() {
    let home = tmp("merge");
    let path = settings_path(&home);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "[]").unwrap();
    let mut proposed = Settings::default();
    proposed.apps.insert("grok".into(), AppPref::default());
    let got = load_or_propose(&path, proposed.clone());
    assert!(got.apps.contains_key("grok"));

    let mut existing = Settings::default();
    existing.model = "cat".into();
    existing.apps.insert(
        "claude".into(),
        AppPref {
            enabled: false,
            ..Default::default()
        },
    );
    save(&path, &existing).unwrap();
    let merged = load_or_propose(&path, proposed);
    assert_eq!(merged.model, "cat");
    assert!(!merged.apps.get("claude").unwrap().enabled);
    assert!(merged.apps.contains_key("grok"));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn merge_fills_empty_model_and_empty_override_ignored() {
    let mut existing = Settings::default();
    existing.model.clear();
    let mut proposed = Settings::default();
    proposed.model = "parrot".into();
    let m = merge_propose(&existing, proposed);
    assert_eq!(m.model, "parrot");
    let mut s = Settings::default();
    s.model = "default".into();
    s.apps.insert(
        "grok".into(),
        AppPref {
            enabled: true,
            model: Some("".into()),
            sound_whip: Some("".into()),
            sound_next: Some("".into()),
        },
    );
    assert_eq!(s.model_for(Some("grok")), "default");
    assert_eq!(s.sound_whip_for(Some("grok")), "whip");
    assert_eq!(s.sound_next_for(Some("grok")), "next");
    assert_eq!(s.model_for(None), "default");
    assert_eq!(s.sound_whip_for(None), "whip");
    assert_eq!(s.sound_next_for(None), "next");
}

#[test]
fn home_dir_honors_whipnext_home() {
    let p = tmp("home");
    std::env::set_var("WHIPNEXT_HOME", &p);
    assert_eq!(home_dir(), p);
    std::env::remove_var("WHIPNEXT_HOME");
    let fallback = home_dir();
    assert!(!fallback.as_os_str().is_empty());
    let _ = std::fs::remove_dir_all(&p);
}

#[test]
fn save_fails_when_parent_is_a_file() {
    let home = tmp("savefail");
    let file = home.join("not-a-dir");
    std::fs::write(&file, b"x").unwrap();
    let err = save(&file.join("settings.json"), &Settings::default()).unwrap_err();
    assert!(!err.is_empty());
    let invalid = std::path::PathBuf::from("\\\\.\\NUL\\whipnext-nope\\settings.json");
    let _ = save(&invalid, &Settings::default());
    let _ = save(std::path::Path::new(""), &Settings::default());
    assert!(whipnext::settings::is_ai_ide("copilot"));
    assert!(whipnext::settings::is_ai_ide("vscode-copilot"));
    assert!(whipnext::settings::is_ai_ide("windsurf"));
    assert!(!whipnext::settings::is_ai_ide("civ6"));
    let _ = std::fs::remove_dir_all(&home);
}
