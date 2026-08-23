use std::fs;
use std::path::PathBuf;

use whipnext::pack::{self, load_pack, sound_bytes};

fn write_png(path: &PathBuf, rgb: [u8; 3]) {
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
    img.save(path).unwrap();
}

#[test]
fn replacing_model_or_sound_file_changes_shipped_loader_results() {
    let root = std::env::temp_dir().join(format!("whipnext-pack-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("models/default")).unwrap();
    fs::create_dir_all(root.join("sounds")).unwrap();
    write_png(&root.join("models/default/still.png"), [10, 20, 30]);
    fs::write(root.join("sounds/whip.wav"), b"WHIP-A").unwrap();
    fs::write(
        root.join("models/default/manifest.json"),
        r#"{"id":"default","still":"still.png"}"#,
    )
    .unwrap();

    let pack = load_pack(&root, "default").unwrap();
    let still_a = pack.model.still_bytes.clone();
    let whip_a = sound_bytes(&pack, "whip").unwrap();
    assert_eq!(whip_a, b"WHIP-A");

    write_png(&root.join("models/default/still.png"), [200, 10, 10]);
    fs::write(root.join("sounds/whip.wav"), b"WHIP-B").unwrap();

    let pack2 = load_pack(&root, "default").unwrap();
    assert_ne!(pack2.model.still_bytes, still_a);
    assert_eq!(sound_bytes(&pack2, "whip").unwrap(), b"WHIP-B");
    assert_ne!(sound_bytes(&pack2, "whip").unwrap(), whip_a);
    let _ = pack::models_dir(&root);
    let _ = fs::remove_dir_all(&root);
}
