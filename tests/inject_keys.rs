use whipnext::inject::{typed_keys, TypedKey, MODIFIER_VKS, NEXT_PROMPT, NEXT_SCANS, NEXT_VKS};

#[test]
fn next_vks_are_n_e_x_t() {
    let chars: String = NEXT_VKS
        .iter()
        .map(|vk| char::from_u32(*vk as u32).unwrap())
        .collect();
    assert_eq!(chars.to_ascii_lowercase(), NEXT_PROMPT);
    assert_ne!(chars.to_ascii_lowercase(), "nicht");
    assert_eq!(NEXT_SCANS, [0x31, 0x12, 0x2D, 0x14]);
    assert_eq!(NEXT_VKS.len(), NEXT_SCANS.len());
    assert_eq!(
        typed_keys(NEXT_PROMPT),
        vec![
            TypedKey::Vk(0x4E),
            TypedKey::Vk(0x45),
            TypedKey::Vk(0x58),
            TypedKey::Vk(0x54),
        ]
    );
    assert_eq!(typed_keys("keep going")[4], TypedKey::Vk(b' ' as u16));
    assert_eq!(typed_keys("a1")[1], TypedKey::Vk(b'1' as u16));
    assert_eq!(typed_keys("да")[0], TypedKey::Unicode('д'));
    assert_eq!(typed_keys("N")[0], TypedKey::Unicode('N'));
    assert!(MODIFIER_VKS.contains(&0x12));
    assert!(MODIFIER_VKS.contains(&0xA0));
    assert!(MODIFIER_VKS.contains(&0xA3));
    assert!(MODIFIER_VKS.contains(&0xA5));
    assert!(whipnext::inject::INJECT_ATTACH_MS >= 50);
    assert!(whipnext::inject::INJECT_SETTLE_MS >= 100);
    assert!(whipnext::inject::INJECT_MODIFIER_MS >= 30);
    assert!(whipnext::inject::INJECT_DOWN_UP_MS >= 8);
    assert!(whipnext::inject::INJECT_KEY_MS >= 20);
    assert!(whipnext::inject::INJECT_ENTER_MS >= 40);
    assert!(whipnext::inject::INJECT_PASTE_MS >= 80);
    assert!(whipnext::inject::is_vk_char('n'));
    assert!(whipnext::inject::is_vk_char(' '));
    assert!(!whipnext::inject::is_vk_char('д'));
    assert!(!whipnext::inject::is_vk_char(','));
    assert_eq!(
        whipnext::inject::type_method("next", false),
        whipnext::inject::TypeMethod::Paste
    );
    assert_eq!(
        whipnext::inject::type_method("шевелись плотва, дальше", true),
        whipnext::inject::TypeMethod::Paste
    );
    assert_eq!(
        whipnext::inject::type_method("шевелись плотва, дальше", false),
        whipnext::inject::TypeMethod::Paste
    );
    assert_eq!(whipnext::inject::layout_key(-1), None);
    assert_eq!(whipnext::inject::layout_key(0x4E), Some((0x4E, false)));
    assert_eq!(whipnext::inject::layout_key(0x014E), Some((0x4E, true)));
    assert_eq!(
        whipnext::inject::layout_key(0x024E),
        None,
        "Ctrl chord must not be typed"
    );
    assert!(whipnext::inject::layout_ok_for("да", |ch| {
        if ch == 'д' || ch == 'а' {
            Some((1, false))
        } else {
            None
        }
    }));
    assert!(!whipnext::inject::layout_ok_for("да!", |ch| {
        if ch == '!' {
            None
        } else {
            Some((1, false))
        }
    }));
    let dir = std::env::temp_dir().join(format!("whipnext-dev-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let sink = whipnext::inject::DevSink {
        log_path: dir.join("dev.log"),
        last_path: dir.join("dev-last-inject.json"),
    };
    whipnext::inject::write_dev(
        &sink,
        &serde_json::json!({"event":"inject-dev","payload":"да"}),
    );
    let log = std::fs::read_to_string(&sink.log_path).unwrap();
    assert!(log.contains("inject-dev"));
    assert!(log.contains("да"));
    let last = std::fs::read_to_string(&sink.last_path).unwrap();
    assert!(last.contains("payload"));
    let from_home = whipnext::inject::dev_sink(&dir);
    assert!(from_home.log_path.ends_with("dev.log"));
    assert!(from_home.last_path.ends_with("dev-last-inject.json"));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        !whipnext::inject::focus_needs_restore(false),
        "visible fullscreen must not SW_RESTORE"
    );
    assert!(whipnext::inject::focus_needs_restore(true));
    let (bin, args) = whipnext::inject::herdr_prompt_command("herdr", "pane-1");
    assert_eq!(bin, "herdr");
    assert!(args.iter().any(|a| a == "next"));
}

#[test]
fn execute_live_payload_is_next_not_slash() {
    use whipnext::inject::{execute_live, NEXT_PROMPT};
    use whipnext::kind::Kind;
    use whipnext::session::Target;
    let mut seen = String::new();
    let mut pid_fn = |_: u32, text: &str| {
        seen = text.to_string();
        Ok(())
    };
    let mut herdr_fn = |_: &str, _: &[String]| Ok(());
    execute_live(
        &Target {
            kind: Kind::Grok,
            session_id: None,
            pid: Some(8),
            pane_id: None,
            herdr_name: None,
        },
        &mut pid_fn,
        &mut herdr_fn,
    )
    .unwrap();
    assert_eq!(seen, NEXT_PROMPT);
    assert!(!seen.starts_with('/'));
}

#[test]
fn execute_live_payload_uses_character_phrase() {
    use whipnext::inject::execute_live_payload;
    use whipnext::kind::Kind;
    use whipnext::session::Target;
    let mut seen = String::new();
    let mut pid_fn = |_: u32, text: &str| {
        seen = text.to_string();
        Ok(())
    };
    let mut herdr_fn = |_: &str, _: &[String]| Ok(());
    execute_live_payload(
        &Target {
            kind: Kind::Grok,
            session_id: None,
            pid: Some(8),
            pane_id: None,
            herdr_name: None,
        },
        "keep going",
        &mut pid_fn,
        &mut herdr_fn,
    )
    .unwrap();
    assert_eq!(seen, "keep going");
    assert!(!seen.starts_with('/'));
}
