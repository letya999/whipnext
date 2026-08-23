use std::path::PathBuf;
use whipnext::overlay::load_frame_rgba;

#[test]
fn shipped_frames_keep_transparent_pixels_not_black_fill() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/frames/idle_00.png");
    let rgba = load_frame_rgba(
        &path,
        whipnext::overlay::W as u32,
        whipnext::overlay::H as u32,
    );
    assert_eq!(rgba.len(), whipnext::overlay::W * whipnext::overlay::H * 4);
    let transparent = rgba.chunks_exact(4).filter(|p| p[3] == 0).count();
    let opaque = rgba.chunks_exact(4).filter(|p| p[3] > 200).count();
    assert!(
        transparent > 1000,
        "surround must stay alpha 0, got {transparent}"
    );
    assert!(opaque > 1000, "character must stay opaque, got {opaque}");
    // no forced charcoal fill on transparent samples
    let filled = rgba
        .chunks_exact(4)
        .filter(|p| p[3] == 0 && p[0] == 0x10 && p[1] == 0x10 && p[2] == 0x14)
        .count();
    assert_eq!(filled, 0);
}

#[cfg(windows)]
#[test]
fn premultiply_zero_alpha_is_zero_rgb() {
    let out = whipnext::overlay::premultiply_bgra(&[10, 20, 30, 0, 255, 0, 0, 255]);
    assert_eq!(&out[0..4], &[0, 0, 0, 0]);
    assert_eq!(out[3], 0);
    assert_eq!(out[7], 255);
}
