use std::fs;
use std::path::PathBuf;

mod common;

use whipnext::pack::{
    catalog_dir, default_manifest, frame_paths, gltf_source, import_character,
    import_character_bytes, import_sound_bytes, list_lines, list_model_ids, list_models,
    list_models_merged, list_phrases, list_sound_stems, load_model, load_pack, load_sounds,
    media_kind_of, model_is_3d, phrase_from_manifest, read_manifest, resolve_pack_root, slug_id,
    sound_bytes, sounds_dir, unique_id, ImportBytesFile, ImportSpec,
};

fn png(path: &PathBuf) {
    image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255]))
        .save(path)
        .unwrap();
}

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("whipnext-pack-edge-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(p.join("models/m")).unwrap();
    fs::create_dir_all(p.join("sounds")).unwrap();
    p
}

#[test]
fn missing_idle_and_short_action_and_bad_manifest() {
    let root = root("idle");
    png(&root.join("models/m/still.png"));
    png(&root.join("models/m/action_00.png"));
    fs::write(
        root.join("models/m/manifest.json"),
        r#"{"id":"m","still":"still.png","idle":["idle_00.png"],"action":["action_00.png"]}"#,
    )
    .unwrap();
    let err = frame_paths(&root, "m").unwrap_err();
    assert!(
        err.contains("no idle") || err.contains("at least 2"),
        "{err}"
    );
    png(&root.join("models/m/idle_00.png"));
    png(&root.join("models/m/idle_01.png"));
    fs::write(
        root.join("models/m/manifest.json"),
        r#"{"id":"m","still":"still.png","idle":["idle_00.png","idle_01.png"],"action":["action_00.png"]}"#,
    )
    .unwrap();
    let err = frame_paths(&root, "m").unwrap_err();
    assert!(err.contains("at least 2"), "{err}");
    fs::write(root.join("models/m/manifest.json"), "not-json").unwrap();
    assert!(read_manifest(&root, "m").is_err());
    assert!(frame_paths(&root, "m").is_err());
    assert!(load_model(&root, "m").is_err());
    assert!(load_pack(&root, "m").is_err());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn no_manifest_uses_defaults_and_missing_still_errors() {
    let root = root("nom");
    let m = default_manifest("ghost");
    assert_eq!(m.still, "still.png");
    assert_eq!(read_manifest(&root, "ghost").unwrap().id, "ghost");
    assert!(load_model(&root, "ghost").is_err());
    png(&root.join("models/m/still.png"));
    fs::write(
        root.join("models/m/manifest.json"),
        r#"{"id":"m","name":"","still":"still.png","video":"missing.mp4"}"#,
    )
    .unwrap();
    let model = load_model(&root, "m").unwrap();
    assert!(model.video_path.is_none());
    let infos = list_models(&root);
    assert!(infos.iter().any(|i| i.id == "m" && i.name == "m"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn sounds_skip_dirs_and_missing_name_errors() {
    let root = root("snd");
    fs::write(root.join("models/not-a-model.txt"), b"x").unwrap();
    assert!(list_model_ids(&root).contains(&"m".to_string()));
    assert!(!list_model_ids(&root).iter().any(|id| id.ends_with(".txt")));
    fs::create_dir_all(root.join("sounds/nested")).unwrap();
    fs::write(root.join("sounds/whip.wav"), b"W").unwrap();
    let map = load_sounds(&root).unwrap();
    assert!(map.contains_key("whip"));
    assert!(!map.contains_key("nested"));
    let pack = {
        png(&root.join("models/m/still.png"));
        load_pack(&root, "m").unwrap()
    };
    assert!(sound_bytes(&pack, "missing").is_err());
    assert_eq!(sound_bytes(&pack, "whip").unwrap(), b"W");
    assert_eq!(list_sound_stems(&root), vec!["whip".to_string()]);
    assert!(load_sounds(&root.join("nope")).unwrap().is_empty());
    let _ = sounds_dir(&root);
    let empty = std::env::temp_dir().join(format!("whipnext-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&empty);
    assert!(list_model_ids(&empty).is_empty());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn list_models_skips_broken_manifest_and_sound_read_errors() {
    let root = root("broken");
    png(&root.join("models/m/still.png"));
    fs::write(root.join("models/m/manifest.json"), "not-json").unwrap();
    fs::create_dir_all(root.join("models/ok")).unwrap();
    png(&root.join("models/ok/still.png"));
    fs::write(
        root.join("models/ok/manifest.json"),
        r#"{"id":"ok","still":"still.png"}"#,
    )
    .unwrap();
    let infos = list_models(&root);
    assert!(infos.iter().any(|i| i.id == "ok"));
    assert!(!infos.iter().any(|i| i.id == "m"));
    fs::write(root.join("sounds/whip.wav"), b"W").unwrap();
    let pack = load_pack(&root, "ok").unwrap();
    fs::remove_file(root.join("sounds/whip.wav")).unwrap();
    assert!(sound_bytes(&pack, "whip").is_err());
    let _ = fs::remove_dir_all(&root);
}

fn write_png(path: &PathBuf) {
    image::RgbaImage::from_pixel(6, 6, image::Rgba([9, 8, 7, 255]))
        .save(path)
        .unwrap();
}

#[test]
fn import_copies_into_catalog_not_source_and_reloads() {
    let src = std::env::temp_dir().join(format!("whipnext-import-src-{}", std::process::id()));
    let catalog = std::env::temp_dir().join(format!("whipnext-import-cat-{}", std::process::id()));
    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&catalog);
    fs::create_dir_all(&src).unwrap();
    write_png(&src.join("hero.png"));
    write_png(&src.join("idle.png"));
    write_png(&src.join("action.png"));
    fs::write(
        &src.join("clip.gif"),
        b"GIF89a\x01\x00\x01\x00\x00\x00\x00;",
    )
    .unwrap();
    fs::write(
        &src.join("mark.svg"),
        b"<svg xmlns='http://www.w3.org/2000/svg' width='4' height='4'></svg>",
    )
    .unwrap();
    fs::write(&src.join("hero.glb"), b"glTF\x02\x00\x00\x00").unwrap();
    fs::write(&src.join("hit.wav"), b"RIFF....WAVEfmt ").unwrap();

    assert_eq!(media_kind_of(&src.join("clip.gif")), "gif");
    assert_eq!(media_kind_of(&src.join("mark.svg")), "svg");
    assert_eq!(media_kind_of(&src.join("hero.glb")), "3d");

    let spec = ImportSpec {
        name: "Arena Hero".into(),
        phrase: Some("keep going".into()),
        media: Some(src.join("hero.glb")),
        still: Some(src.join("hero.png")),
        idle: Some(src.join("idle.png")),
        action: Some(src.join("action.png")),
        hit_sound: Some(src.join("hit.wav")),
        ..ImportSpec::default()
    };
    let m = import_character(&catalog, &spec).unwrap();
    assert_eq!(m.id, "arena-hero");
    assert_eq!(m.name, "Arena Hero");
    assert_eq!(m.phrase.as_deref(), Some("keep going"));
    assert_eq!(m.sound_action.as_deref(), Some("hit"));
    let still = catalog.join("models/arena-hero").join(&m.still);
    assert!(still.is_file());
    assert!(
        !still.starts_with(&src),
        "catalog copy must not be the picker path"
    );
    assert!(catalog.join("sounds/hit.wav").is_file());
    assert_eq!(
        fs::read(catalog.join("sounds/hit.wav")).unwrap(),
        fs::read(src.join("hit.wav")).unwrap()
    );
    let (idle, action) = frame_paths(&catalog, "arena-hero").unwrap();
    assert!(idle.len() >= 2);
    assert!(action.len() >= 2);
    let loaded = load_model(&catalog, "arena-hero").unwrap();
    assert!(loaded.still_path.starts_with(&catalog));
    assert_eq!(phrase_from_manifest(&catalog, "arena-hero"), "keep going");

    let gif = import_character(
        &catalog,
        &ImportSpec {
            name: "Gif One".into(),
            media: Some(src.join("clip.gif")),
            ..ImportSpec::default()
        },
    )
    .unwrap();
    assert_eq!(gif.media_kind.as_deref(), Some("gif"));
    assert!(
        catalog
            .join("models")
            .join(&gif.id)
            .join("source.gif")
            .is_file()
            || catalog
                .join("models")
                .join(&gif.id)
                .join(&gif.still)
                .is_file()
    );

    let svg = import_character(
        &catalog,
        &ImportSpec {
            name: "Svg Mark".into(),
            media: Some(src.join("mark.svg")),
            phrase: Some("/slash".into()),
            ..ImportSpec::default()
        },
    )
    .unwrap();
    assert_eq!(svg.media_kind.as_deref(), Some("svg"));
    assert_eq!(svg.phrase.as_deref(), Some("next"));

    let bytes = import_character_bytes(
        &catalog,
        "From Bytes",
        Some("advance"),
        &[ImportBytesFile {
            role: "still".into(),
            name: "x.png".into(),
            bytes: fs::read(src.join("hero.png")).unwrap(),
        }],
    )
    .unwrap();
    assert_eq!(bytes.phrase.as_deref(), Some("advance"));
    assert!(load_model(&catalog, &bytes.id).is_ok());

    let merged = list_models_merged(&root("empty-bundled"), &catalog);
    assert!(merged.iter().any(|i| i.id == "arena-hero"));
    assert_eq!(resolve_pack_root(&src, &catalog, "arena-hero"), catalog);
    assert_eq!(slug_id("Arena Hero"), "arena-hero");
    assert_eq!(unique_id(&catalog, "arena-hero"), "arena-hero-2");
    assert!(catalog_dir(std::path::Path::new("h")).ends_with("catalog"));
    let err = import_character(&catalog, &ImportSpec::default()).unwrap_err();
    assert!(err.contains("name"));
    let err = import_character(
        &catalog,
        &ImportSpec {
            name: "x".into(),
            ..ImportSpec::default()
        },
    )
    .unwrap_err();
    assert!(err.contains("pick"));
    let err = import_character(
        &catalog,
        &ImportSpec {
            name: "x".into(),
            still: Some(src.join("missing.png")),
            ..ImportSpec::default()
        },
    )
    .unwrap_err();
    assert!(err.contains("missing"));
    assert_eq!(whipnext::inject::sanitize_phrase("  "), "next");
    assert_eq!(whipnext::inject::sanitize_phrase("/x"), "next");
    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&catalog);
}

#[test]
fn bundled_examples_cover_image_svg_gif_and_3d() {
    let root = whipnext::pack::default_pack_root();
    let models = list_models(&root);
    let get = |id: &str| models.iter().find(|m| m.id == id).unwrap();
    assert_eq!(get("manager").media_kind, "image");
    assert_eq!(get("lash").media_kind, "svg");
    assert_eq!(get("feather").media_kind, "gif");
    assert_eq!(get("capybara").media_kind, "3d");
    assert!(get("capybara").source.contains("capybara.gltf"));
    assert!(
        get("capybara").still.is_empty(),
        "3D catalog tile is not a PNG, got {}",
        get("capybara").still
    );
    assert_eq!(get("capybara").idle, vec!["idle".to_string()]);
    assert_eq!(get("capybara").action, vec!["headbutt-cycle".to_string()]);
    assert!(get("lash").source.ends_with("still.svg"));
    let (idle, action) = frame_paths(&root, "feather").unwrap();
    assert_eq!(idle.len(), 1);
    assert_eq!(action.len(), 1);
    assert_eq!(media_kind_of(&action[0]), "gif");
    assert!(load_model(&root, "manager").is_ok());
    assert!(load_model(&root, "lash").is_ok());
    let phrases = list_phrases(&root);
    assert_eq!(phrases.len(), 4);
    assert!(phrases.iter().any(|p| p.text == "шевелись плотва, дальше"));
    assert!(phrases.iter().any(|p| p.text == "go ahead"));
    assert_eq!(
        phrases.iter().find(|p| p.id == "plotva").unwrap().sound,
        "whip"
    );
    assert!(model_is_3d(&root, "capybara"));
    assert!(!model_is_3d(&root, "manager"));
    let src = gltf_source(&root, "capybara").unwrap();
    assert!(src.ends_with("capybara.gltf"));
    let (idle3, act3) = frame_paths(&root, "capybara").unwrap();
    assert_eq!(idle3.len(), 1);
    assert_eq!(media_kind_of(&idle3[0]), "3d");
    assert_eq!(act3[0], idle3[0]);
}

#[test]
fn import_sound_copies_wav_into_catalog() {
    let catalog = std::env::temp_dir().join(format!("whipnext-snd-{}", std::process::id()));
    let _ = fs::remove_dir_all(&catalog);
    let wav = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/whip.wav");
    let bytes = fs::read(&wav).unwrap();
    let stem = import_sound_bytes(&catalog, "My Hit.wav", &bytes).unwrap();
    assert_eq!(stem, "my-hit");
    assert!(catalog.join("sounds/my-hit.wav").is_file());
    assert!(import_sound_bytes(&catalog, "x.wav", b"nope").is_err());
    let _ = fs::remove_dir_all(catalog.join("sounds"));
    fs::write(catalog.join("sounds"), b"file-not-dir").unwrap();
    let _ = import_sound_bytes(&catalog, "z.wav", &bytes);
    let _ = fs::remove_dir_all(&catalog);
}

#[test]
fn single_gif_counts_as_action_sequence() {
    let root = root("onegif");
    png(&root.join("models/m/idle_00.png"));
    let gif =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/models/feather/action.gif");
    fs::copy(&gif, root.join("models/m/hit.gif")).unwrap();
    fs::write(
        root.join("models/m/manifest.json"),
        r#"{"id":"m","still":"idle_00.png","idle":["idle_00.png"],"action":["hit.gif"]}"#,
    )
    .unwrap();
    let (_idle, action) = frame_paths(&root, "m").unwrap();
    assert_eq!(action.len(), 1);
    fs::write(
        root.join("models/m/manifest.json"),
        r#"{"id":"m","still":"idle_00.png","idle":["idle_00.png"],"action":["idle_00.png"]}"#,
    )
    .unwrap();
    let err = frame_paths(&root, "m").unwrap_err();
    assert!(err.contains("at least 2"), "{err}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn lines_merge_phrases_json_and_loose_wavs() {
    let bundled = root("lines-b");
    fs::write(
        bundled.join("phrases.json"),
        r#"[{"id":"hi","locale":"en","text":"hello"}]"#,
    )
    .unwrap();
    fs::write(bundled.join("sounds/hi.wav"), b"RIFF....WAVE").unwrap();
    fs::write(bundled.join("sounds/extra.wav"), b"RIFF....WAVE").unwrap();
    let got = list_phrases(&bundled);
    assert_eq!(got.iter().find(|p| p.id == "hi").unwrap().sound, "hi");
    let catalog = std::env::temp_dir().join(format!("whipnext-lines-c-{}", std::process::id()));
    let _ = fs::remove_dir_all(&catalog);
    fs::create_dir_all(catalog.join("sounds")).unwrap();
    fs::write(catalog.join("sounds/honk.wav"), b"RIFF....WAVE").unwrap();
    fs::write(
        catalog.join("phrases.json"),
        r#"[{"id":"custom","locale":"ru","text":"ещё","sound":"honk"}]"#,
    )
    .unwrap();
    let lines = list_lines(&bundled, &catalog);
    assert!(lines.iter().any(|p| p.id == "hi" && p.sound == "hi"));
    assert!(lines.iter().any(|p| p.id == "custom" && p.text == "ещё"));
    assert!(lines.iter().any(|p| p.id == "extra" && p.text == "next"));
    fs::write(catalog.join("phrases.json"), "not-json").unwrap();
    let lines = list_lines(&bundled, &catalog);
    assert!(lines.iter().any(|p| p.id == "extra"));
    fs::write(bundled.join("phrases.json"), "nope").unwrap();
    assert_eq!(list_phrases(&bundled).len(), 0);
    let _ = fs::remove_dir_all(&bundled);
    let _ = fs::remove_dir_all(&catalog);
}

#[test]
fn import_3d_copies_gltf_bin_without_png_frames() {
    let src = common::nugget_fixture();
    let catalog = std::env::temp_dir().join(format!("whipnext-user-cat-{}", std::process::id()));
    let _ = fs::remove_dir_all(&catalog);
    assert!(
        src.join("nugget/nugget.gltf").is_file(),
        "user-drop character missing"
    );
    assert!(src.join("honk.wav").is_file(), "user-drop sound missing");
    let m = import_character(
        &catalog,
        &ImportSpec {
            name: "Nugget".into(),
            phrase: Some("go on".into()),
            media: Some(src.join("nugget/nugget.gltf")),
            hit_sound: Some(src.join("honk.wav")),
            ..ImportSpec::default()
        },
    )
    .unwrap();
    assert_eq!(m.id, "nugget");
    assert_eq!(m.media_kind.as_deref(), Some("3d"));
    let dir = catalog.join("models/nugget");
    assert!(dir.join("source.gltf").is_file());
    assert!(dir.join("nugget.bin").is_file());
    assert!(!dir.join("still.png").is_file());
    assert!(!dir.join("idle_00.png").is_file());
    assert_eq!(m.sound_action.as_deref(), Some("honk"));
    assert!(catalog.join("sounds/honk.wav").is_file());
    assert!(model_is_3d(&catalog, "nugget"));
    let loaded = load_model(&catalog, "nugget").unwrap();
    assert!(loaded.still_path.ends_with("source.gltf"));
    let bytes = import_character_bytes(
        &catalog,
        "Nugget Two",
        None,
        &[
            ImportBytesFile {
                role: "media".into(),
                name: "nugget.gltf".into(),
                bytes: fs::read(src.join("nugget/nugget.gltf")).unwrap(),
            },
            ImportBytesFile {
                role: "bin".into(),
                name: "nugget.bin".into(),
                bytes: fs::read(src.join("nugget/nugget.bin")).unwrap(),
            },
        ],
    )
    .unwrap();
    assert_eq!(bytes.media_kind.as_deref(), Some("3d"));
    assert!(catalog
        .join("models")
        .join(&bytes.id)
        .join("nugget.bin")
        .is_file());
    assert!(!catalog
        .join("models")
        .join(&bytes.id)
        .join("still.png")
        .is_file());
    let stem = import_sound_bytes(
        &catalog,
        "honk.wav",
        &fs::read(src.join("honk.wav")).unwrap(),
    )
    .unwrap();
    assert_eq!(stem, "honk");
    let lines = list_lines(&whipnext::pack::default_pack_root(), &catalog);
    assert!(lines.iter().any(|p| p.sound == "honk"));
    let _ = fs::remove_dir_all(&catalog);
}

#[test]
fn gltf_source_scans_dir_and_source_files() {
    let root = root("scan3d");
    fs::create_dir_all(root.join("models/blob")).unwrap();
    fs::write(root.join("models/blob/weird.glb"), b"glTF").unwrap();
    fs::write(
        root.join("models/blob/manifest.json"),
        r#"{"id":"blob","still":"nope.png"}"#,
    )
    .unwrap();
    let src = gltf_source(&root, "blob").unwrap();
    assert!(src.ends_with("weird.glb"));
    assert!(model_is_3d(&root, "blob"));
    fs::create_dir_all(root.join("models/empty")).unwrap();
    fs::write(
        root.join("models/empty/manifest.json"),
        r#"{"id":"empty","still":"still.png","media_kind":"3d"}"#,
    )
    .unwrap();
    assert!(model_is_3d(&root, "empty"));
    assert!(gltf_source(&root, "empty").is_none());
    assert!(frame_paths(&root, "empty").unwrap_err().contains("no glTF"));
    assert!(!model_is_3d(&root, "ghost-missing"));
    fs::create_dir_all(root.join("models/src3")).unwrap();
    png(&root.join("models/src3/still.png"));
    fs::write(root.join("models/src3/source.gltf"), b"{}\n").unwrap();
    fs::write(
        root.join("models/src3/manifest.json"),
        r#"{"id":"src3","still":"still.png","source":"source.gltf"}"#,
    )
    .unwrap();
    assert!(model_is_3d(&root, "src3"));
    fs::create_dir_all(root.join("models/prev")).unwrap();
    png(&root.join("models/prev/preview.png"));
    fs::write(root.join("models/prev/cap.gltf"), b"{}\n").unwrap();
    fs::write(
        root.join("models/prev/manifest.json"),
        r#"{"id":"prev","name":"","still":"cap.gltf","media_kind":"3d"}"#,
    )
    .unwrap();
    let prev = list_models(&root);
    let p = prev.iter().find(|m| m.id == "prev").unwrap();
    assert!(p.still.contains("preview.png"));
    assert_eq!(p.name, "prev");
    fs::create_dir_all(root.join("models/blank")).unwrap();
    png(&root.join("models/blank/still.png"));
    fs::write(
        root.join("models/blank/manifest.json"),
        r#"{"id":"blank","still":"","media_kind":"image"}"#,
    )
    .unwrap();
    let blank = list_models(&root)
        .into_iter()
        .find(|m| m.id == "blank")
        .unwrap();
    assert!(blank.still.contains("still.png"));
    fs::create_dir_all(root.join("models/biny")).unwrap();
    fs::write(root.join("models/biny/x.bin"), b"xx").unwrap();
    fs::write(
        root.join("models/biny/manifest.json"),
        r#"{"id":"biny","still":"x.bin"}"#,
    )
    .unwrap();
    let biny = list_models(&root)
        .into_iter()
        .find(|m| m.id == "biny")
        .unwrap();
    assert_eq!(biny.media_kind, "image");
    assert_eq!(slug_id("***"), "character");
    assert_eq!(slug_id("a--b"), "a-b");
    fs::create_dir_all(root.join("models/n")).unwrap();
    fs::create_dir_all(root.join("models/n-2")).unwrap();
    assert_eq!(unique_id(&root, "n"), "n-3");
    assert!(list_phrases(&root.join("no-phrases-here")).is_empty());
    assert_eq!(
        whipnext::inject::sanitize_phrase("/slash"),
        whipnext::inject::NEXT_PROMPT
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn import_bytes_roles_and_empty_sound_stem() {
    let catalog = std::env::temp_dir().join(format!("whipnext-roles-{}", std::process::id()));
    let _ = fs::remove_dir_all(&catalog);
    let pngb = {
        let p = catalog.join("_t.png");
        fs::create_dir_all(&catalog).unwrap();
        png(&p);
        fs::read(&p).unwrap()
    };
    let wav =
        fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/paw.wav"))
            .unwrap();
    let src = common::nugget_fixture();
    let gltf = fs::read(src.join("nugget/nugget.gltf")).unwrap();
    let bin = fs::read(src.join("nugget/nugget.bin")).unwrap();
    let m = import_character_bytes(
        &catalog,
        "Mix",
        None,
        &[
            ImportBytesFile {
                role: "still".into(),
                name: String::new(),
                bytes: pngb.clone(),
            },
            ImportBytesFile {
                role: "still".into(),
                name: "ok.png".into(),
                bytes: vec![],
            },
            ImportBytesFile {
                role: "still".into(),
                name: "../x.png".into(),
                bytes: pngb.clone(),
            },
            ImportBytesFile {
                role: "media".into(),
                name: "clip.mp4".into(),
                bytes: b"mp4".to_vec(),
            },
            ImportBytesFile {
                role: "idle".into(),
                name: "i.png".into(),
                bytes: pngb.clone(),
            },
            ImportBytesFile {
                role: "action".into(),
                name: "a.png".into(),
                bytes: pngb.clone(),
            },
            ImportBytesFile {
                role: "sound".into(),
                name: ".wav".into(),
                bytes: wav.clone(),
            },
            ImportBytesFile {
                role: "media".into(),
                name: "nugget.gltf".into(),
                bytes: gltf,
            },
            ImportBytesFile {
                role: "bin".into(),
                name: "nugget.bin".into(),
                bytes: bin,
            },
            ImportBytesFile {
                role: "media".into(),
                name: "nested/notes.txt".into(),
                bytes: b"kept with the source folder".to_vec(),
            },
        ],
    )
    .unwrap();
    assert_eq!(m.media_kind.as_deref(), Some("3d"));
    assert!(catalog
        .join("models/mix/_source/nested/notes.txt")
        .is_file());
    let src = std::env::temp_dir().join(format!("whipnext-dotwav-{}", std::process::id()));
    let _ = fs::remove_dir_all(&src);
    fs::create_dir_all(&src).unwrap();
    png(&src.join("g.png"));
    fs::write(src.join(".wav"), &wav).unwrap();
    let two = import_character(
        &catalog,
        &ImportSpec {
            name: "Dot Wav".into(),
            still: Some(src.join("g.png")),
            hit_sound: Some(src.join(".wav")),
            ..ImportSpec::default()
        },
    )
    .unwrap();
    assert!(two.sound_action.is_some());

    fs::write(src.join("bare"), b"xx").unwrap();
    assert_eq!(media_kind_of(&src.join("bare")), "file");
    let _ = import_character(
        &catalog,
        &ImportSpec {
            name: "Bare File".into(),
            still: Some(src.join("bare")),
            hit_sound: Some(src.join("missing-hit.wav")),
            ..ImportSpec::default()
        },
    );

    let sounds_file = catalog.join("sndfile");
    fs::create_dir_all(sounds_file.join("models")).unwrap();
    fs::write(sounds_file.join("sounds"), b"not-dir").unwrap();
    let _ = import_character(
        &sounds_file,
        &ImportSpec {
            name: "Snd File".into(),
            still: Some(src.join("g.png")),
            ..ImportSpec::default()
        },
    );
    let dest_dir = catalog.join("copydest");
    fs::create_dir_all(dest_dir.join("sounds/paw.wav")).unwrap();
    fs::create_dir_all(dest_dir.join("models")).unwrap();
    let _ = import_character(
        &dest_dir,
        &ImportSpec {
            name: "Copy Fail".into(),
            still: Some(src.join("g.png")),
            hit_sound: Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/paw.wav"),
            ),
            ..ImportSpec::default()
        },
    );
    let blocked = catalog.join("blocked-cat");
    fs::write(&blocked, b"not-a-dir").unwrap();
    assert!(import_character(
        &blocked,
        &ImportSpec {
            name: "Nope".into(),
            still: Some(src.join("g.png")),
            ..ImportSpec::default()
        },
    )
    .is_err());

    let still_gltf = root("still3d");
    fs::create_dir_all(still_gltf.join("models/g")).unwrap();
    let src = common::nugget_fixture();
    fs::copy(
        src.join("nugget/nugget.gltf"),
        still_gltf.join("models/g/hero.gltf"),
    )
    .unwrap();
    fs::copy(
        src.join("nugget/nugget.bin"),
        still_gltf.join("models/g/nugget.bin"),
    )
    .unwrap();
    fs::write(
        still_gltf.join("models/g/manifest.json"),
        r#"{"id":"g","still":"hero.gltf"}"#,
    )
    .unwrap();
    assert!(model_is_3d(&still_gltf, "g"));
    assert_eq!(media_kind_of(&still_gltf.join("models/g/hero.gltf")), "3d");

    let wav_as_still = import_character_bytes(
        &catalog,
        "Wav Still",
        None,
        &[
            ImportBytesFile {
                role: "still".into(),
                name: "a.png".into(),
                bytes: pngb.clone(),
            },
            ImportBytesFile {
                role: "still".into(),
                name: "beep.wav".into(),
                bytes: wav.clone(),
            },
            ImportBytesFile {
                role: "still".into(),
                name: "a..png".into(),
                bytes: pngb.clone(),
            },
            ImportBytesFile {
                role: "still".into(),
                name: "notes.txt".into(),
                bytes: b"hello".to_vec(),
            },
        ],
    )
    .unwrap();
    assert_eq!(wav_as_still.sound_action.as_deref(), Some("beep"));
    let _ = fs::remove_dir_all(&catalog);
    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&still_gltf);
}

#[test]
fn video_set_import_replace_and_unplayable_format() {
    use whipnext::pack::{playable_clip_kind, replace_character, replace_character_bytes};

    let catalog = std::env::temp_dir().join(format!("whipnext-vid-cat-{}", std::process::id()));
    let src = std::env::temp_dir().join(format!("whipnext-vid-src-{}", std::process::id()));
    let _ = fs::remove_dir_all(&catalog);
    let _ = fs::remove_dir_all(&src);
    fs::create_dir_all(&src).unwrap();
    let idle = src.join("idle.mp4");
    let action = src.join("action.mp4");
    whipnext::video::write_test_clip(&idle, 3, "red").unwrap();
    whipnext::video::write_test_clip(&action, 4, "blue").unwrap();
    let wav = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/paw.wav");
    let m = import_character(
        &catalog,
        &ImportSpec {
            name: "Vid Girl".into(),
            idle: Some(idle.clone()),
            action: Some(action.clone()),
            still: Some(idle.clone()),
            hit_sound: Some(wav.clone()),
            ..ImportSpec::default()
        },
    )
    .unwrap();
    assert_eq!(m.id, "vid-girl");
    assert_eq!(m.media_kind.as_deref(), Some("video"));
    assert_eq!(m.sound_action.as_deref(), Some("paw"));
    let (idle_p, act_p) = frame_paths(&catalog, "vid-girl").unwrap();
    assert_eq!(idle_p.len(), 1);
    assert_eq!(act_p.len(), 1);
    assert_eq!(media_kind_of(&idle_p[0]), "video");
    assert!(catalog.join("sounds/paw.wav").is_file());

    let clap = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/whip.wav");
    let idle2 = src.join("idle2.mp4");
    whipnext::video::write_test_clip(&idle2, 3, "green").unwrap();
    let replaced = replace_character(
        &catalog,
        "vid-girl",
        &ImportSpec {
            idle: Some(idle2.clone()),
            action: Some(action.clone()),
            still: Some(idle2.clone()),
            hit_sound: Some(clap.clone()),
            ..ImportSpec::default()
        },
    )
    .unwrap();
    assert_eq!(replaced.id, "vid-girl");
    assert_eq!(replaced.sound_action.as_deref(), Some("whip"));
    let man = read_manifest(&catalog, "vid-girl").unwrap();
    assert!(man.idle.iter().any(|n| n.contains("idle")));
    let bytes = fs::read(&idle2).unwrap();
    let again = replace_character_bytes(
        &catalog,
        "vid-girl",
        None,
        &[ImportBytesFile {
            role: "idle".into(),
            name: "later.mp4".into(),
            bytes,
        }],
    )
    .unwrap();
    assert_eq!(again.id, "vid-girl");

    fs::write(src.join("hero.fbx"), b"fbx").unwrap();
    let err = playable_clip_kind(&src.join("hero.fbx")).unwrap_err();
    assert!(err.contains("not playable"), "{err}");
    let err = import_character(
        &catalog,
        &ImportSpec {
            name: "Fake 3d".into(),
            still: Some(src.join("hero.fbx")),
            ..ImportSpec::default()
        },
    )
    .unwrap_err();
    assert!(err.contains("not playable"), "{err}");
    assert_eq!(media_kind_of(&src.join("hero.fbx")), "unsupported");
    fs::write(src.join("bad.mp4"), b"not a video").unwrap();
    let err = import_character(
        &catalog,
        &ImportSpec {
            name: "Broken Vid".into(),
            media: Some(src.join("bad.mp4")),
            ..ImportSpec::default()
        },
    )
    .unwrap_err();
    assert!(err.contains("not playable"), "{err}");
    let _ = fs::remove_dir_all(&catalog);
    let _ = fs::remove_dir_all(&src);
}

#[test]
fn phrases_and_layout_are_single_sourced() {
    let pack_src =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/pack.rs")).unwrap();
    assert!(
        !pack_src.contains("шевелись плотва"),
        "phrase catalog lives in pack JSON, not Rust"
    );
    let phrases = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/phrases.json"),
    )
    .unwrap();
    assert!(phrases.contains("шевелись плотва"));
    let overlay =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/overlay.rs"))
            .unwrap();
    assert!(overlay.contains("TICK_MS"));
    assert!(!overlay.contains("from_millis(140)"));
    let app =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/ui/app.js"))
            .unwrap();
    assert!(app.contains("layout.tick_ms"));
    assert_eq!(
        whipnext::inject::sanitize_phrase(""),
        whipnext::inject::NEXT_PROMPT
    );
}
