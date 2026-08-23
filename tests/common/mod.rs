use std::fs;
use std::path::PathBuf;

pub fn nugget_fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!("whipnext-nugget-fixture-{}", std::process::id()));
    let dir = root.join("nugget");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&dir).unwrap();

    let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/models/capybara");
    let gltf = fs::read_to_string(bundled.join("capybara.gltf"))
        .unwrap()
        .replace("capybara.bin", "nugget.bin");
    fs::write(dir.join("nugget.gltf"), gltf).unwrap();
    fs::copy(bundled.join("capybara.bin"), dir.join("nugget.bin")).unwrap();
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/sounds/paw.wav"),
        root.join("honk.wav"),
    )
    .unwrap();
    root
}
