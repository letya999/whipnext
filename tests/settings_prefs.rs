use whipnext::pack;
use whipnext::settings::{AppPref, OverlayShowMode, Settings};

#[test]
fn per_app_model_and_sounds_override_global() {
    let mut s = Settings::default();
    s.model = "default".into();
    s.sound_whip = "whip".into();
    s.sound_next = "next".into();
    s.apps.insert(
        "grok".into(),
        AppPref {
            enabled: true,
            model: Some("alt".into()),
            sound_whip: Some("crack".into()),
            sound_next: Some("go".into()),
        },
    );
    s.apps.insert(
        "claude".into(),
        AppPref {
            enabled: false,
            ..Default::default()
        },
    );
    assert!(s.app_enabled("grok"));
    assert!(!s.app_enabled("claude"));
    assert!(!s.app_enabled("civ6"));
    assert_eq!(s.model_for(Some("grok")), "alt");
    assert_eq!(s.model_for(Some("claude")), "default");
    s.model = "cat".into();
    assert_eq!(s.model_for(Some("claude")), "cat");
    assert_eq!(s.model_for(None), "cat");
    assert_eq!(s.sound_whip_for(Some("grok")), "crack");
    assert_eq!(s.sound_next_for(Some("grok")), "go");
    assert_eq!(s.sound_next_for(Some("claude")), "next");
    assert_eq!(s.resolved_sound_next(Some("grok"), Some("bang")), "go");
    assert_eq!(s.resolved_sound_next(Some("claude"), Some("bang")), "bang");
    assert_eq!(s.resolved_sound_next(Some("pi"), None), "next");
    assert_eq!(
        whipnext::settings::OverlayShowMode::default(),
        OverlayShowMode::Always
    );
    assert!(!Settings::default().dev_mode);
}

#[test]
fn pack_ships_four_models_with_action_frames() {
    let root = pack::default_pack_root();
    let ids = pack::list_model_ids(&root);
    for id in ["default", "commissar", "cat", "parrot", "manager", "lash"] {
        assert!(ids.iter().any(|x| x == id), "missing model {id}");
        let (idle, action) = pack::frame_paths(&root, id).unwrap();
        assert!(!idle.is_empty(), "{id} idle");
        assert!(action.len() >= 2, "{id} action");
    }
    for id in ["feather", "capybara"] {
        assert!(ids.iter().any(|x| x == id), "missing model {id}");
        assert!(pack::frame_paths(&root, id).is_ok(), "{id} frames");
    }
}

#[test]
fn harness_character_and_phrase_independent() {
    let mut s = Settings::default();
    s.model = "default".into();
    s.phrases.insert("hero-x".into(), "keep going".into());
    s.phrases.insert("slashy".into(), "/nope".into());
    s.apps.insert(
        "grok".into(),
        AppPref {
            enabled: true,
            model: Some("hero-x".into()),
            ..Default::default()
        },
    );
    s.apps.insert(
        "claude".into(),
        AppPref {
            enabled: true,
            model: Some("hero-y".into()),
            ..Default::default()
        },
    );
    assert_eq!(s.model_for(Some("grok")), "hero-x");
    assert_eq!(s.model_for(Some("claude")), "hero-y");
    assert_eq!(s.model_for(Some("pi")), "default");
    assert_eq!(s.phrase_for("hero-x"), "keep going");
    assert_eq!(s.phrase_for("hero-y"), "next");
    assert_eq!(s.phrase_for("slashy"), "next");
    assert_eq!(s.phrase_for_surface(Some("grok")), "keep going");
    assert_eq!(s.phrase_for_surface(Some("claude")), "next");
    s.overlay_scale = 150;
    s.control_size = 80;
    s.overlay_mode = OverlayShowMode::WhileHeld;
    s.overlay_key = "ctrl".into();
    let raw = serde_json::to_string(&s).unwrap();
    let back: Settings = serde_json::from_str(&raw).unwrap();
    assert_eq!(back.overlay_scale, 150);
    assert_eq!(back.control_size, 80);
    assert_eq!(back.overlay_mode, OverlayShowMode::WhileHeld);
    assert_eq!(back.overlay_key, "ctrl");
    assert_eq!(back.phrases.get("hero-x").unwrap(), "keep going");
}

#[test]
fn hit_sound_follows_character_unless_app_override() {
    let mut s = Settings::default();
    s.sound_whip = "whip".into();
    s.apps.insert(
        "grok".into(),
        AppPref {
            enabled: true,
            model: Some("hero-x".into()),
            ..Default::default()
        },
    );
    assert_eq!(s.resolved_sound_whip(Some("grok"), Some("bang")), "bang");
    s.model_sounds.insert("hero-x".into(), "paw".into());
    assert_eq!(s.resolved_sound_whip(Some("grok"), Some("bang")), "paw");
    assert_eq!(s.resolved_sound_whip(Some("claude"), Some("bang")), "bang");
    assert_eq!(s.resolved_sound_whip(Some("pi"), None), "whip");
    s.apps.get_mut("grok").unwrap().sound_whip = Some("crack".into());
    assert_eq!(s.resolved_sound_whip(Some("grok"), Some("bang")), "crack");
    assert_eq!(s.app_sound_whip_override(Some("grok")), Some("crack"));
    assert!(s.app_sound_whip_override(Some("claude")).is_none());
}
