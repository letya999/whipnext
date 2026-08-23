use std::sync::Mutex;

use whipnext::detect::FakeProbe;
use whipnext::settings::{AppPref, OverlayShowMode, Settings};
use whipnext::ui::{
    apply_imported, apply_ipc, apply_settings, decode_b64, mime, parse_ipc, proposed_from,
    serve_asset, serve_asset_ex, snapshot, write_launch_log, Ipc,
};

fn tmp(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("whipnext-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn serve_asset_index_png_traversal_and_missing() {
    let assets = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let index = serve_asset(&assets, "/");
    assert_eq!(index.status, 200);
    assert!(index.mime.contains("html"));
    assert!(std::str::from_utf8(&index.bytes)
        .unwrap()
        .contains("whipnext"));
    let png = serve_asset(&assets, "/icon-256.png");
    assert_eq!(png.status, 200);
    assert_eq!(png.mime, "image/png");
    assert!(!png.bytes.is_empty());
    let bad = serve_asset(&assets, "/../Cargo.toml");
    assert_eq!(bad.status, 400);
    let miss = serve_asset(&assets, "/nope.bin");
    assert_eq!(miss.status, 404);
    assert_eq!(mime(std::path::Path::new("a.wav")), "audio/wav");
    assert_eq!(mime(std::path::Path::new("a.jpg")), "image/jpeg");
    assert_eq!(mime(std::path::Path::new("a.jpeg")), "image/jpeg");
    assert_eq!(
        mime(std::path::Path::new("a.js")),
        "text/javascript; charset=utf-8"
    );
    assert_eq!(
        mime(std::path::Path::new("a.css")),
        "text/css; charset=utf-8"
    );
    assert_eq!(mime(std::path::Path::new("a.json")), "application/json");
    assert_eq!(mime(std::path::Path::new("a.ico")), "image/x-icon");
    assert_eq!(
        mime(std::path::Path::new("a.bin")),
        "application/octet-stream"
    );
    assert_eq!(mime(std::path::Path::new("a.glb")), "model/gltf-binary");
    assert_eq!(mime(std::path::Path::new("a.gltf")), "model/gltf+json");
    assert_eq!(mime(std::path::Path::new("a.gif")), "image/gif");
    assert_eq!(mime(std::path::Path::new("a.svg")), "image/svg+xml");
    let html = std::str::from_utf8(&index.bytes).unwrap();
    assert!(html.contains("/ui/atoms/"));
    assert!(html.contains("/ui/molecules/"));
    assert!(html.contains("/ui/organisms/"));
    assert!(html.contains("/ui/layouts/"));
    assert!(html.contains("/ui/i18n.js"));
    assert!(html.contains("lang_ru"));
    assert!(html.contains("org-harness"));
    assert!(!html.contains("org-sounds"));
    assert!(!html.contains("org-phrases"));
    assert!(html.contains("/ui/gltf-view.js"));
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/ui");
    assert!(root.join("atoms/atoms.css").is_file());
    assert!(root.join("atoms/atoms.js").is_file());
    assert!(root.join("molecules/molecules.css").is_file());
    assert!(root.join("molecules/molecules.js").is_file());
    assert!(root.join("organisms/organisms.css").is_file());
    assert!(root.join("organisms/organisms.js").is_file());
    assert!(root.join("layouts/settings-layout.css").is_file());
    let app_js = std::fs::read_to_string(root.join("app.js")).unwrap();
    assert!(app_js.contains("apps[id].model"));
    assert!(app_js.contains("make_default"));
    assert!(app_js.contains("selectedId"));
    assert!(app_js.contains("edit_phrase"));
    assert!(app_js.contains("new_phrase"));
    assert!(app_js.contains("GltfView.play"));
    assert!(app_js.contains("layout.tick_ms"));
    assert!(app_js.contains("replace-bytes"));
    assert!(!app_js.contains("require("));
    assert!(app_js.contains("filesToMsgs"));
    assert!(app_js.contains("stage3d-keep"));
    assert!(app_js.contains("window.GltfView.stop()"));
    let gltf_js = std::fs::read_to_string(root.join("gltf-view.js")).unwrap();
    assert!(!gltf_js.contains("require("));
    assert!(!gltf_js.contains("module.exports"));
    assert!(gltf_js.contains("function play("));
    assert!(gltf_js.contains("headbutt-cycle"));
    assert!(gltf_js.contains("idleName"));
    assert!(gltf_js.contains("actionName"));
    assert!(
        !app_js.contains("apps[id].sound_whip"),
        "harness character pick must not copy hit sound onto a per-app override"
    );
    let i18n = serve_asset(&assets, "/ui/i18n.js");
    assert_eq!(i18n.status, 200);
    let i18n_txt = std::str::from_utf8(&i18n.bytes).unwrap();
    assert!(i18n_txt.contains("rescan"));
    assert!(i18n_txt.contains("Найти харнессы"));
    assert!(i18n_txt.contains("Заменить папку"));
    assert!(i18n_txt.contains("Сделать дефолтным"));
    assert!(i18n_txt.contains("devMode"));
    assert!(i18n_txt.contains("Режим отладки"));
    assert!(i18n_txt.contains("default:"));
    assert!(!i18n_txt.contains("shared:"));
    let org = std::fs::read_to_string(root.join("organisms/organisms.js")).unwrap();
    assert!(org.contains("data-add"));
    assert!(org.contains("redetect"));
    assert!(org.contains("dev_mode"));
    assert!(org.contains("harness-assign"));
    assert!(org.contains("resource-path"));
    assert!(org.contains("open_sound_folder"));
    assert!(!org.contains("sound-catalog"));
    assert!(!org.contains("phrase-catalog"));
    assert!(!org.contains("data-open-catalog"));
    assert!(org.contains("play_sound"));
    assert!(org.contains("animationStages"));
    assert!(org.contains("file_hit_sound"));
    assert!(org.contains("edit_phrase"));
    assert!(org.contains("make_default"));
    assert!(org.contains("t('default')"));
    assert!(!org.contains("t('shared')"));
    assert!(!org.contains("replace_assets"));
    assert!(!org.contains("play_voice"));
    let atoms = serve_asset(&assets, "/ui/atoms/atoms.css");
    assert_eq!(atoms.status, 200);
    let layout = serve_asset(&assets, "/ui/layouts/settings-layout.css");
    assert_eq!(layout.status, 200);
}

#[test]
fn parse_ipc_commands_and_garbage() {
    assert_eq!(parse_ipc(r#"{"cmd":"ready"}"#), Ipc::Ready);
    assert_eq!(parse_ipc(r#"{"cmd":"redetect"}"#), Ipc::Redetect);
    assert_eq!(parse_ipc(r#"{"cmd":"pack"}"#), Ipc::Ignore);
    assert_eq!(parse_ipc(r#"{"cmd":"quit"}"#), Ipc::Quit);
    assert_eq!(
        parse_ipc(r#"{"cmd":"play","sound":"whip"}"#),
        Ipc::Play("whip".into())
    );
    assert_eq!(parse_ipc(r#"{"cmd":"play"}"#), Ipc::Ignore);
    assert_eq!(parse_ipc("not-json"), Ipc::Ignore);
    assert_eq!(
        parse_ipc(r#"{"cmd":"set","settings":{}}"#).clone(),
        parse_ipc(r#"{"cmd":"set","settings":{}}"#)
    );
    match parse_ipc(r#"{"cmd":"set","settings":{"model":"cat"}}"#) {
        Ipc::Set(s) => assert_eq!(s.model, "cat"),
        other => panic!("{other:?}"),
    }
    assert_eq!(parse_ipc(r#"{"cmd":"set","settings":1}"#), Ipc::Ignore);
    match parse_ipc(r#"{"cmd":"import-sound","name":"clap.wav","b64":"AAAA"}"#) {
        Ipc::ImportSound { name, b64 } => {
            assert_eq!(name, "clap.wav");
            assert_eq!(b64, "AAAA");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(parse_ipc(r#"{"cmd":"import-sound"}"#), Ipc::Ignore);
}

#[test]
fn apply_ipc_set_and_redetect_persist() {
    let home = tmp("ipc");
    let path = home.join("settings.json");
    let live = Mutex::new(Settings::default());
    let mut proposed = Settings::default();
    proposed.apps.insert("grok".into(), AppPref::default());
    let set = Settings {
        model: "parrot".into(),
        ..Settings::default()
    };
    assert!(apply_ipc(Ipc::Set(set.clone()), &path, &live, proposed.clone()).is_none());
    assert_eq!(live.lock().unwrap().model, "parrot");
    let on_disk: Settings = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk.model, "parrot");
    apply_ipc(Ipc::Redetect, &path, &live, proposed).unwrap();
    assert!(live.lock().unwrap().apps.contains_key("grok"));
    assert!(apply_ipc(Ipc::Quit, &path, &live, Settings::default()).is_none());
    assert!(apply_ipc(Ipc::Ready, &path, &live, Settings::default()).is_none());
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn snapshot_lists_shipped_models() {
    let root = whipnext::pack::default_pack_root();
    let snap = snapshot(&Settings::default(), &root);
    let models = snap["models"].as_array().unwrap();
    let ids: Vec<_> = models.iter().map(|m| m["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"default"));
    assert!(ids.contains(&"cat"));
    assert!(ids.contains(&"manager"));
    assert!(ids.contains(&"lash"));
    assert!(ids.contains(&"feather"));
    assert!(ids.contains(&"capybara"));
    let by = |id: &str| models.iter().find(|m| m["id"] == id).unwrap();
    assert!(models
        .iter()
        .all(|m| m["name"].as_str().unwrap().is_ascii()));
    assert_eq!(by("manager")["media_kind"], "image");
    assert_eq!(by("lash")["media_kind"], "svg");
    assert_eq!(by("feather")["media_kind"], "gif");
    assert_eq!(by("capybara")["media_kind"], "3d");
    assert!(by("capybara")["source"]
        .as_str()
        .unwrap()
        .contains("capybara.gltf"));
    assert_eq!(snap["logo"], "/icon-256.png");
    assert_eq!(
        snap["layout"]["overlay_w"],
        whipnext::layout::OVERLAY_W as u64
    );
    assert_eq!(snap["layout"]["tick_ms"], whipnext::layout::TICK_MS);
    assert_eq!(
        snap["layout"]["scale_min"],
        whipnext::layout::SCALE_MIN as u64
    );
    assert!(snap["herdr"].is_array());
    let assets = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    assert_eq!(
        serve_asset(&assets, "/pack/models/lash/still.svg").mime,
        "image/svg+xml"
    );
    assert_eq!(
        serve_asset(&assets, "/pack/models/feather/idle.gif").status,
        200
    );
    assert_eq!(
        serve_asset(&assets, "/pack/models/capybara/capybara.gltf").status,
        200
    );
    assert_eq!(
        serve_asset(&assets, "/pack/models/capybara/capybara.gltf").mime,
        "model/gltf+json"
    );
    let phrases = snap["phrases"].as_array().unwrap();
    assert_eq!(phrases.len(), 4);
    let texts: Vec<_> = phrases.iter().filter_map(|p| p["text"].as_str()).collect();
    assert!(texts.contains(&"шевелись плотва, дальше"));
    assert!(texts.contains(&"ну что там?"));
    assert!(texts.contains(&"next"));
    assert!(texts.contains(&"go ahead"));
    let plotva = phrases.iter().find(|p| p["id"] == "plotva").unwrap();
    assert_eq!(plotva["sound"], "whip");
    assert_eq!(snap["sounds"].as_array().unwrap().len(), 4);
    assert!(snap["sound_files"].as_array().unwrap()[0]["path"]
        .as_str()
        .unwrap()
        .ends_with(".wav"));
    let capy = by("capybara");
    assert_eq!(capy["idle"][0], "idle");
    assert_eq!(capy["action"][0], "headbutt-cycle");
    assert_eq!(capy["preview"], "");
    assert_eq!(
        serve_asset(&assets, "/pack/models/capybara/capybara.bin").status,
        200
    );
}

#[test]
fn proposed_from_probe_and_launch_log() {
    let mut probe = FakeProbe::default();
    probe.path_dirs.push(std::path::PathBuf::from("/bin"));
    probe.files.push(std::path::PathBuf::from("/bin/pi"));
    probe.running = vec![(1, "cursor.exe".into()), (2, "CivilizationVI.exe".into())];
    let s = proposed_from(&probe);
    assert!(s.apps.contains_key("pi"));
    assert!(s.apps.contains_key("cursor"));
    assert!(!s.apps.contains_key("CivilizationVI"));
    let home = tmp("log");
    write_launch_log(&home, "hello");
    let log = std::fs::read_to_string(home.join(".whipnext").join("launch.log")).unwrap();
    assert!(log.contains("hello"));
    let _ = apply_settings(
        &home.join("s.json"),
        &Mutex::new(Settings::default()),
        Settings::default(),
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn snapshot_and_set_round_trip_new_fields() {
    let mut s = Settings::default();
    s.model = "cat".into();
    s.overlay_scale = 125;
    s.control_size = 90;
    s.overlay_mode = OverlayShowMode::HideWhileHeld;
    s.overlay_key = "shift".into();
    s.phrases.insert("cat".into(), "keep going".into());
    s.sound_whip = "paw".into();
    let root = whipnext::pack::default_pack_root();
    let snap = snapshot(&s, &root);
    assert_eq!(snap["settings"]["overlay_scale"], 125);
    assert_eq!(snap["settings"]["control_size"], 90);
    assert_eq!(snap["settings"]["overlay_mode"], "hide_while_held");
    assert_eq!(snap["settings"]["overlay_key"], "shift");
    assert_eq!(snap["settings"]["phrases"]["cat"], "keep going");
    assert_eq!(snap["settings"]["dev_mode"], false);
    s.dev_mode = true;
    let snap = snapshot(&s, &root);
    assert_eq!(snap["settings"]["dev_mode"], true);
    s.locale = "en".into();
    let snap = snapshot(&s, &root);
    assert_eq!(snap["settings"]["locale"], "en");
    let models = snap["models"].as_array().unwrap();
    assert!(models.iter().any(|m| m.get("phrase").is_some()));

    match parse_ipc(&format!(
        r#"{{"cmd":"set","settings":{}}}"#,
        serde_json::to_string(&s).unwrap()
    )) {
        Ipc::Set(got) => {
            assert_eq!(got.overlay_scale, 125);
            assert_eq!(got.phrases.get("cat").unwrap(), "keep going");
            assert_eq!(got.overlay_mode, OverlayShowMode::HideWhileHeld);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn import_ipc_copies_into_settings_catalog() {
    let home = tmp("import-ipc");
    let path = home.join("settings.json");
    let live = Mutex::new(Settings::default());
    let src = home.join("src");
    std::fs::create_dir_all(&src).unwrap();
    image::RgbaImage::from_pixel(4, 4, image::Rgba([2, 3, 4, 255]))
        .save(src.join("a.png"))
        .unwrap();
    let msg = format!(
        r#"{{"cmd":"import","name":"Ipc Hero","phrase":"onward","still":{}}}"#,
        serde_json::to_string(&src.join("a.png")).unwrap()
    );
    match parse_ipc(&msg) {
        Ipc::Import(spec) => {
            assert_eq!(spec.name, "Ipc Hero");
            apply_ipc(Ipc::Import(spec), &path, &live, Settings::default()).unwrap();
        }
        other => panic!("{other:?}"),
    }
    let got = live.lock().unwrap().clone();
    assert_eq!(got.model, "ipc-hero");
    assert_eq!(got.phrases.get("ipc-hero").unwrap(), "onward");
    assert!(home.join("catalog/models/ipc-hero/manifest.json").is_file());
    assert!(!home
        .join("catalog/models/ipc-hero/still.png")
        .starts_with(&src));

    let png = std::fs::read(src.join("a.png")).unwrap();
    let b64 = {
        // encode with the same alphabet the decoder accepts
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        let mut i = 0;
        while i < png.len() {
            let b0 = png[i];
            let b1 = if i + 1 < png.len() { png[i + 1] } else { 0 };
            let b2 = if i + 2 < png.len() { png[i + 2] } else { 0 };
            o.push(T[(b0 >> 2) as usize] as char);
            o.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
            if i + 1 < png.len() {
                o.push(T[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                o.push('=');
            }
            if i + 2 < png.len() {
                o.push(T[(b2 & 63) as usize] as char);
            } else {
                o.push('=');
            }
            i += 3;
        }
        o
    };
    assert_eq!(decode_b64(&b64).unwrap(), png);
    let msg = format!(
        r#"{{"cmd":"import-bytes","name":"B64 Pal","phrase":"go","files":[{{"role":"still","name":"z.png","b64":"{b64}"}}]}}"#
    );
    match parse_ipc(&msg) {
        Ipc::ImportBytes { name, .. } => assert_eq!(name, "B64 Pal"),
        other => panic!("{other:?}"),
    }
    apply_ipc(parse_ipc(&msg), &path, &live, Settings::default()).unwrap();
    assert_eq!(live.lock().unwrap().model, "b64-pal");
    assert_eq!(parse_ipc(r#"{"cmd":"import"}"#), Ipc::Ignore);
    assert_eq!(parse_ipc(r#"{"cmd":"import-bytes"}"#), Ipc::Ignore);
    match parse_ipc(r#"{"cmd":"import-bytes","name":"Solo"}"#) {
        Ipc::ImportBytes { name, files, .. } => {
            assert_eq!(name, "Solo");
            assert!(files.is_empty());
        }
        other => panic!("{other:?}"),
    }
    assert!(decode_b64("abc").is_err());
    assert!(decode_b64("@@@@").is_err());
    assert!(decode_b64("A@AA").is_err());
    assert!(decode_b64("AA@A").is_err());
    assert!(decode_b64("AAA@").is_err());
    let miss =
        parse_ipc(r#"{"cmd":"import","name":"Nope","still":"C:/no-such-whipnext-file.png"}"#);
    apply_ipc(miss, &path, &live, Settings::default());

    let plus = decode_b64("QQ++").unwrap_or_else(|_| decode_b64("QQ+/AA==").unwrap());
    assert!(!plus.is_empty() || decode_b64("QQ==").is_ok());
    assert!(decode_b64("QQ==").is_ok());
    assert!(decode_b64("QQA=").is_ok());
    assert_eq!(
        parse_ipc(r#"{"cmd":"import-bytes","name":"   "}"#),
        Ipc::Ignore
    );
    let skip = parse_ipc(
        r#"{"cmd":"import-bytes","name":"X","files":[{"role":"still","name":"a.png","b64":"@@@"}]}"#,
    );
    match skip {
        Ipc::ImportBytes { files, .. } => assert!(files.is_empty()),
        other => panic!("{other:?}"),
    }
    apply_ipc(
        parse_ipc(r#"{"cmd":"import-sound","name":"clap.wav","b64":"QQ=="}"#),
        &path,
        &live,
        Settings::default(),
    );
    let wav = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/whip.wav"),
    )
    .unwrap();
    let b64 = {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        let mut i = 0;
        while i < wav.len() {
            let b0 = wav[i];
            let b1 = if i + 1 < wav.len() { wav[i + 1] } else { 0 };
            let b2 = if i + 2 < wav.len() { wav[i + 2] } else { 0 };
            o.push(T[(b0 >> 2) as usize] as char);
            o.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
            if i + 1 < wav.len() {
                o.push(T[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                o.push('=');
            }
            if i + 2 < wav.len() {
                o.push(T[(b2 & 63) as usize] as char);
            } else {
                o.push('=');
            }
            i += 3;
        }
        o
    };
    apply_ipc(
        parse_ipc(&format!(
            r#"{{"cmd":"import-sound","name":"boom.wav","b64":"{b64}"}}"#
        )),
        &path,
        &live,
        Settings::default(),
    );
    assert_eq!(
        live.lock()
            .unwrap()
            .model_sounds
            .values()
            .next()
            .map(|s| s.as_str()),
        Some("boom")
    );
    let mut man = whipnext::pack::default_manifest("z");
    man.sound_action = Some("paw".into());
    man.phrase = Some("onward".into());
    apply_imported(&path, &live, man);
    assert_eq!(
        live.lock()
            .unwrap()
            .model_sounds
            .get("z")
            .map(|s| s.as_str()),
        Some("paw")
    );

    let cat = home.join("catalog");
    std::fs::create_dir_all(cat.join("models/x")).unwrap();
    std::fs::write(cat.join("models/x/note.txt"), b"hi").unwrap();
    let assets = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let served = serve_asset_ex(&assets, &cat, "/pack/models/x/note.txt");
    assert_eq!(served.status, 200);
    assert_eq!(served.bytes, b"hi");
    apply_ipc(
        parse_ipc(r#"{"cmd":"import-sound","name":"x.wav","b64":"@@@@"}"#),
        &path,
        &live,
        Settings::default(),
    );
    apply_ipc(
        parse_ipc(r#"{"cmd":"import-bytes","name":"Empty","files":[]}"#),
        &path,
        &live,
        Settings::default(),
    );
    let no_phrase = whipnext::pack::default_manifest("plain");
    apply_imported(&path, &live, no_phrase);
    assert_eq!(
        whipnext::ui::catalog_from_settings_path(std::path::Path::new("settings.json")),
        std::path::PathBuf::from("catalog")
    );
    assert!(decode_b64("+/8A").is_ok() || decode_b64("+/8=").is_ok());
    let poison = std::sync::Mutex::new(Settings::default());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = poison.lock().unwrap();
        panic!("poison");
    }));
    let _ = apply_settings(&path, &poison, Settings::default());
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn replace_bytes_ipc_rewrites_catalog_files_and_hit_sound() {
    let home = tmp("replace-ipc");
    let path = home.join("settings.json");
    let live = Mutex::new(Settings::default());
    let src = home.join("src");
    std::fs::create_dir_all(&src).unwrap();
    image::RgbaImage::from_pixel(6, 6, image::Rgba([4, 5, 6, 255]))
        .save(src.join("a.png"))
        .unwrap();
    let wav = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/paw.wav"),
    )
    .unwrap();
    apply_ipc(
        parse_ipc(&format!(
            r#"{{"cmd":"import","name":"Swap Me","still":{}}}"#,
            serde_json::to_string(&src.join("a.png")).unwrap()
        )),
        &path,
        &live,
        Settings::default(),
    );
    assert_eq!(live.lock().unwrap().model, "swap-me");
    assert!(apply_ipc(
        parse_ipc(r#"{"cmd":"rename","id":"swap-me","name":"Swap Safe"}"#),
        &path,
        &live,
        Settings::default(),
    )
    .is_none());
    assert_eq!(
        whipnext::pack::read_manifest(&home.join("catalog"), "swap-me")
            .unwrap()
            .name,
        "Swap Safe"
    );
    image::RgbaImage::from_pixel(6, 6, image::Rgba([9, 1, 1, 255]))
        .save(src.join("b.png"))
        .unwrap();
    let png = std::fs::read(src.join("b.png")).unwrap();
    let b64_png = {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        let mut i = 0;
        while i < png.len() {
            let b0 = png[i];
            let b1 = if i + 1 < png.len() { png[i + 1] } else { 0 };
            let b2 = if i + 2 < png.len() { png[i + 2] } else { 0 };
            o.push(T[(b0 >> 2) as usize] as char);
            o.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
            if i + 1 < png.len() {
                o.push(T[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                o.push('=');
            }
            if i + 2 < png.len() {
                o.push(T[(b2 & 63) as usize] as char);
            } else {
                o.push('=');
            }
            i += 3;
        }
        o
    };
    let b64_wav = {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        let mut i = 0;
        while i < wav.len() {
            let b0 = wav[i];
            let b1 = if i + 1 < wav.len() { wav[i + 1] } else { 0 };
            let b2 = if i + 2 < wav.len() { wav[i + 2] } else { 0 };
            o.push(T[(b0 >> 2) as usize] as char);
            o.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
            if i + 1 < wav.len() {
                o.push(T[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                o.push('=');
            }
            if i + 2 < wav.len() {
                o.push(T[(b2 & 63) as usize] as char);
            } else {
                o.push('=');
            }
            i += 3;
        }
        o
    };
    match parse_ipc(&format!(
        r#"{{"cmd":"replace-bytes","id":"swap-me","files":[{{"role":"still","name":"b.png","b64":"{b64_png}"}},{{"role":"sound","name":"hit.wav","b64":"{b64_wav}"}}]}}"#
    )) {
        Ipc::ReplaceBytes { id, files, .. } => {
            assert_eq!(id, "swap-me");
            assert_eq!(files.len(), 2);
        }
        other => panic!("{other:?}"),
    }
    apply_ipc(
        parse_ipc(&format!(
            r#"{{"cmd":"replace-bytes","id":"swap-me","files":[{{"role":"still","name":"b.png","b64":"{b64_png}"}},{{"role":"sound","name":"hit.wav","b64":"{b64_wav}"}}]}}"#
        )),
        &path,
        &live,
        Settings::default(),
    );
    let man = whipnext::pack::read_manifest(&home.join("catalog"), "swap-me").unwrap();
    assert_eq!(man.sound_action.as_deref(), Some("hit"));
    assert!(home.join("catalog/sounds/hit.wav").is_file());
    live.lock().unwrap().model = "keep-default".into();
    apply_ipc(
        parse_ipc(r#"{"cmd":"replace-bytes","id":"swap-me","phrase":"keep going","files":[]}"#),
        &path,
        &live,
        Settings::default(),
    );
    let after = live.lock().unwrap().clone();
    assert_eq!(after.model, "keep-default");
    assert_eq!(
        after.phrases.get("swap-me").map(String::as_str),
        Some("keep going")
    );
    assert_eq!(
        whipnext::pack::read_manifest(&home.join("catalog"), "swap-me")
            .unwrap()
            .phrase
            .as_deref(),
        Some("keep going")
    );
    assert_eq!(parse_ipc(r#"{"cmd":"replace-bytes"}"#), Ipc::Ignore);
    let _ = std::fs::remove_dir_all(&home);
}
