use std::path::PathBuf;

#[test]
fn shipped_frames_and_sounds_and_unix_backends_exist() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for name in [
        "assets/frames/idle_00.png",
        "assets/frames/idle_01.png",
        "assets/frames/punch_00.png",
        "assets/frames/punch_01.png",
        "assets/frames/punch_02.png",
        "assets/frames/punch_03.png",
        "assets/sounds/whip.wav",
        "assets/sounds/next.wav",
        "assets/icon.ico",
        "src/linux_tests.rs",
        "src/macos_tests.rs",
    ] {
        let p = root.join(name);
        assert!(p.is_file(), "missing {name}");
        assert!(p.metadata().unwrap().len() > 100, "{name} too small");
    }
    let overlay = std::fs::read_to_string(root.join("src/overlay_unix_tests.rs")).unwrap();
    assert!(
        overlay.contains("none: true"),
        "Unix minifb popup must set none:true or the girl window is invisible"
    );
    let a = std::fs::read(root.join("assets/frames/idle_00.png")).unwrap();
    let b = std::fs::read(root.join("assets/frames/punch_02.png")).unwrap();
    assert_ne!(a, b);
    for name in [
        "assets/pack/models/manager/still.png",
        "assets/pack/models/lash/still.svg",
        "assets/pack/models/lash/still.png",
        "assets/pack/models/feather/idle.gif",
        "assets/pack/models/feather/action.gif",
        "assets/pack/models/capybara/capybara.gltf",
        "assets/pack/models/capybara/capybara.bin",
        "assets/pack/phrases.json",
        "assets/pack/sounds/whip.wav",
        "assets/pack/sounds/paw.wav",
        "assets/pack/sounds/ready.wav",
        "assets/pack/sounds/next.wav",
        "assets/ui/gltf-view.js",
    ] {
        let p = root.join(name);
        assert!(p.is_file(), "missing {name}");
        assert!(p.metadata().unwrap().len() > 100, "{name} too small");
    }
    let cap = root.join("assets/pack/models/capybara");
    for ent in std::fs::read_dir(&cap).unwrap() {
        let name = ent.unwrap().file_name().to_string_lossy().to_lowercase();
        assert!(
            !name.ends_with(".png"),
            "capybara pack must be mesh+clips only, leftover {name}"
        );
    }
}

#[test]
fn next_wav_is_english_pcm_not_default_voice_placeholder() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(root.join("assets/sounds/next.wav")).unwrap();
    assert!(bytes.starts_with(b"RIFF"));
    assert!(bytes[8..12] == *b"WAVE");
    assert!(
        bytes.len() > 40_000,
        "spoken next.wav too short: {}",
        bytes.len()
    );
    let pack = std::fs::read(root.join("assets/pack/sounds/next.wav")).unwrap();
    assert_eq!(bytes, pack);
    let src = std::fs::read_to_string(root.join("src/audio_win_tests.rs")).unwrap();
    assert!(src.contains("SND_ASYNC"));
    assert!(!src.contains("SND_SYNC"));
    assert!(
        !src.contains("SND_NOSTOP"),
        "SND_NOSTOP swallows next.wav while whip still plays"
    );
    assert!(src.contains("-> bool"));
}

#[test]
fn sound_file_skips_tiny_catalog_and_uses_pack() {
    let home = std::env::temp_dir().join(format!("whipnext-sndhome-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let cat = home.join(".whipnext").join("catalog").join("sounds");
    std::fs::create_dir_all(&cat).unwrap();
    std::fs::write(cat.join("whip.wav"), b"nope").unwrap();
    std::env::set_var("WHIPNEXT_HOME", &home);
    assert!(!whipnext::audio::usable_wav(&cat.join("whip.wav")));
    let got = whipnext::audio::sound_file("whip");
    std::env::remove_var("WHIPNEXT_HOME");
    let bytes = std::fs::read(&got).unwrap();
    assert!(
        bytes.starts_with(b"RIFF"),
        "expected pack wav, got {}",
        got.display()
    );
    assert!(whipnext::audio::usable_wav(&got));
    let real = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/whip.wav");
    std::fs::copy(&real, cat.join("whip.wav")).unwrap();
    std::env::set_var("WHIPNEXT_HOME", &home);
    let cataloged = whipnext::audio::sound_file("whip");
    std::env::remove_var("WHIPNEXT_HOME");
    assert!(cataloged.ends_with("whip.wav"));
    assert!(whipnext::audio::usable_wav(&cataloged));
    let _ = std::fs::remove_dir_all(&home);
}
