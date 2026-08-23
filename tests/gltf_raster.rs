use std::fs;
use std::path::{Path, PathBuf};

use whipnext::gltf::{
    clip_names, gltf_sidecars, load, overlay_frames, raster, raster_file, raster_pose, sample_clip,
    Clip, Mesh,
};

fn tmp(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("whipnext-gltf-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn opaque(px: &[u8]) -> bool {
    px.chunks(4).any(|p| p[3] > 24)
}

fn pack_f32(vals: &[f32]) -> Vec<u8> {
    vals.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn triangle_bin() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend(pack_f32(&[0.0, 0.9, 0.0, -0.9, -0.8, 0.0, 0.9, -0.8, 0.0]));
    b.extend(pack_f32(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]));
    b.extend([0u8, 1, 2]);
    b.extend(pack_f32(&[0.0, 0.4, 0.8]));
    b.extend(pack_f32(&[
        0.0, 0.0, 0.0, 1.0, 0.0, 0.2, 0.0, 0.98, 0.0, 0.0, 0.0, 1.0,
    ]));
    b.extend(pack_f32(&[0.0, 0.0, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0]));
    b
}

fn triangle_gltf(uri: &str, extras: &str) -> String {
    format!(
        r#"{{
      "asset": {{"version":"2.0"}},
      "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0, "NORMAL": 1}}, "indices": 2, "material": 0}}]}}],
      "materials": [{{"pbrMetallicRoughness": {{"baseColorFactor": [0.2, 0.7, 0.3, 1]}}}}],
      "accessors": [
        {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.9,-0.8,0], "max": [0.9,0.9,0]}},
        {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"}},
        {{"bufferView": 2, "componentType": 5121, "count": 3, "type": "SCALAR"}},
        {{"bufferView": 3, "componentType": 5126, "count": 3, "type": "SCALAR", "min": [0], "max": [0.8]}},
        {{"bufferView": 4, "componentType": 5126, "count": 3, "type": "VEC4"}},
        {{"bufferView": 5, "componentType": 5126, "count": 3, "type": "VEC3"}}
      ],
      "bufferViews": [
        {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
        {{"buffer": 0, "byteOffset": 36, "byteLength": 36}},
        {{"buffer": 0, "byteOffset": 72, "byteLength": 3}},
        {{"buffer": 0, "byteOffset": 75, "byteLength": 12}},
        {{"buffer": 0, "byteOffset": 87, "byteLength": 48}},
        {{"buffer": 0, "byteOffset": 135, "byteLength": 36}}
      ],
      "animations": [{{
        "name": "hop",
        "channels": [
          {{"sampler": 0, "target": {{"node": 0, "path": "rotation"}}}},
          {{"sampler": 1, "target": {{"node": 0, "path": "translation"}}}},
          {{"sampler": 0, "target": {{"node": 0, "path": "scale"}}}},
          {{"sampler": 9, "target": {{"node": 0, "path": "rotation"}}}}
        ],
        "samplers": [
          {{"input": 3, "output": 4, "interpolation": "LINEAR"}},
          {{"input": 3, "output": 5, "interpolation": "STEP"}}
        ]
      }}],
      "buffers": [{{"byteLength": 171, "uri": "{uri}"}}]
      {extras}
    }}"#
    )
}

fn write_triangle(dir: &Path, stem: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let gltf = dir.join(format!("{stem}.gltf"));
    let bin = dir.join(format!("{stem}.bin"));
    fs::write(&bin, triangle_bin()).unwrap();
    fs::write(&gltf, triangle_gltf(&format!("{stem}.bin"), "")).unwrap();
    gltf
}

fn wrap_glb(json: &str, bin: &[u8]) -> Vec<u8> {
    fn pad4(n: usize) -> usize {
        (4 - n % 4) % 4
    }
    let mut json_bytes = json.as_bytes().to_vec();
    json_bytes.extend(std::iter::repeat(b' ').take(pad4(json_bytes.len())));
    let mut bin_bytes = bin.to_vec();
    bin_bytes.extend(std::iter::repeat(0u8).take(pad4(bin_bytes.len())));
    let mut out = Vec::new();
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    let total = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
    out.extend((total as u32).to_le_bytes());
    out.extend((json_bytes.len() as u32).to_le_bytes());
    out.extend(0x4E4F534Au32.to_le_bytes());
    out.extend(&json_bytes);
    out.extend((bin_bytes.len() as u32).to_le_bytes());
    out.extend(0x004E4942u32.to_le_bytes());
    out.extend(&bin_bytes);
    out
}

fn visible_count(px: &[u8]) -> usize {
    px.chunks(4).filter(|p| p[3] > 24).count()
}

#[test]
fn capybara_mesh_rasters_and_clips_differ() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/models/capybara/capybara.gltf");
    let mesh = load(&path).unwrap();
    let names = clip_names(&mesh);
    assert!(names.iter().any(|n| n == "idle"), "{names:?}");
    assert!(names.iter().any(|n| n == "headbutt-cycle"), "{names:?}");
    let still = raster(&mesh, 64, 80, 0.4, None, 0.0);
    assert!(opaque(&still), "rest pose should be visible");
    let idle = raster(&mesh, 64, 80, 0.0, Some("idle"), 0.5);
    let hit = raster(&mesh, 64, 80, 0.0, Some("headbutt-cycle"), 0.8);
    assert!(opaque(&idle) && opaque(&hit));
    assert_ne!(idle, hit, "idle and hit poses must not be identical");
    let (idle_f, act_f) =
        overlay_frames(&mesh, Some("idle"), Some("headbutt-cycle"), 48, 48).unwrap();
    assert_eq!(idle_f.len(), 2);
    assert_eq!(act_f.len(), 4);
    assert!(opaque(&idle_f[0].1) && opaque(&act_f[1].1));
    let via_file = raster_file(&path, 32, 32);
    assert!(opaque(&via_file));
    assert!(visible_count(&via_file) > 20);
}

#[test]
fn load_errors_and_glb_roundtrip() {
    let dir = tmp("err");
    assert!(load(&dir.join("missing.gltf")).is_err());
    fs::write(dir.join("bad.gltf"), "not-json").unwrap();
    assert!(load(&dir.join("bad.gltf")).unwrap_err().contains("json"));
    fs::write(dir.join("tiny.glb"), b"glTF").unwrap();
    assert!(load(&dir.join("tiny.glb")).unwrap_err().contains("small"));
    fs::write(dir.join("magic.glb"), b"NOPE........").unwrap();
    assert!(load(&dir.join("magic.glb")).unwrap_err().contains("magic"));
    let mut ver = b"glTF".to_vec();
    ver.extend(1u32.to_le_bytes());
    ver.extend(12u32.to_le_bytes());
    fs::write(dir.join("ver.glb"), &ver).unwrap();
    assert!(load(&dir.join("ver.glb")).unwrap_err().contains("version"));
    let mut trunc = b"glTF".to_vec();
    trunc.extend(2u32.to_le_bytes());
    trunc.extend(40u32.to_le_bytes());
    trunc.extend(20u32.to_le_bytes());
    trunc.extend(0x4E4F534Au32.to_le_bytes());
    trunc.extend(b"{}");
    fs::write(dir.join("trunc.glb"), &trunc).unwrap();
    assert!(load(&dir.join("trunc.glb")).is_err());

    let gltf = write_triangle(&dir, "tri");
    let mesh = load(&gltf).unwrap();
    assert!(opaque(&raster(&mesh, 40, 40, 0.2, Some("hop"), 0.2)));
    assert!(opaque(&raster(&mesh, 40, 40, 0.2, Some("nope"), 0.0)));
    let glb = wrap_glb(&fs::read_to_string(&gltf).unwrap(), &triangle_bin());
    let glb_path = dir.join("tri.glb");
    fs::write(&glb_path, &glb).unwrap();
    let gmesh = load(&glb_path).unwrap();
    assert!(opaque(&raster(&gmesh, 24, 24, 0.1, None, 0.0)));

    let mut extra = wrap_glb("{}", &[]);
    extra.extend(4u32.to_le_bytes());
    extra.extend(0x12345678u32.to_le_bytes());
    extra.extend([1, 2, 3, 4]);
    fs::write(dir.join("nojson-ish.glb"), extra).unwrap();
    let _ = load(&dir.join("nojson-ish.glb"));

    let mut header = b"glTF".to_vec();
    header.extend(2u32.to_le_bytes());
    header.extend(20u32.to_le_bytes());
    fs::write(dir.join("emptychunk.glb"), header).unwrap();
    assert!(load(&dir.join("emptychunk.glb"))
        .unwrap_err()
        .contains("json"));

    fs::write(
        dir.join("data.gltf"),
        triangle_gltf("data:application/octet-stream,AA", ""),
    )
    .unwrap();
    assert!(load(&dir.join("data.gltf"))
        .unwrap_err()
        .contains("data uri"));
    fs::write(dir.join("nouri.gltf"), triangle_gltf("", "")).unwrap();
    assert!(
        load(&dir.join("nouri.gltf")).is_err()
            || raster_file(&dir.join("nouri.gltf"), 8, 8).len() == 256
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn parser_covers_optional_fields_and_index_types() {
    let dir = tmp("idx");
    let mut bin = triangle_bin();
    bin.extend([0u8, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
    let json = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5125, "count": 3, "type": "SCALAR"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 171, "byteLength": 12}
      ],
      "buffers": [{"byteLength": 183, "uri": "u32.bin"}]
    }"#;
    fs::write(dir.join("u32.bin"), &bin).unwrap();
    fs::write(dir.join("u32.gltf"), json).unwrap();
    let mesh = load(&dir.join("u32.gltf")).unwrap();
    assert_eq!(mesh.idx, vec![0, 1, 2]);
    assert!(opaque(&raster(&mesh, 32, 32, 0.0, None, 0.0)));

    let json16 = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "accessors": [
        {"bufferView": 0, "byteOffset": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 36, "byteLength": 6}
      ],
      "buffers": [{"byteLength": 42, "uri": "u16.bin"}]
    }"#;
    let mut b16 = pack_f32(&[0.0, 0.8, 0.0, -0.8, -0.7, 0.0, 0.8, -0.7, 0.0]);
    b16.extend([0u8, 0, 1, 0, 2, 0]);
    fs::write(dir.join("u16.bin"), &b16).unwrap();
    fs::write(dir.join("u16.gltf"), json16).unwrap();
    assert!(opaque(&raster(
        &load(&dir.join("u16.gltf")).unwrap(),
        20,
        20,
        0.3,
        None,
        0.0
    )));

    let seq = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}
      ],
      "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}],
      "buffers": [{"byteLength": 36, "uri": "seq.bin"}]
    }"#;
    fs::write(
        dir.join("seq.bin"),
        pack_f32(&[0.0, 0.8, 0.0, -0.8, -0.7, 0.0, 0.8, -0.7, 0.0]),
    )
    .unwrap();
    fs::write(dir.join("seq.gltf"), seq).unwrap();
    let sm = load(&dir.join("seq.gltf")).unwrap();
    assert_eq!(sm.idx, vec![0, 1, 2]);

    fs::write(dir.join("nomesh.gltf"), r#"{"asset":{"version":"2.0"}}"#).unwrap();
    assert!(load(&dir.join("nomesh.gltf"))
        .unwrap_err()
        .contains("meshes"));
    fs::write(
        dir.join("noprim.gltf"),
        r#"{"asset":{"version":"2.0"},"meshes":[{}]}"#,
    )
    .unwrap();
    assert!(load(&dir.join("noprim.gltf"))
        .unwrap_err()
        .contains("primitives"));
    fs::write(
        dir.join("noattr.gltf"),
        r#"{"asset":{"version":"2.0"},"meshes":[{"primitives":[{}]}]}"#,
    )
    .unwrap();
    assert!(load(&dir.join("noattr.gltf"))
        .unwrap_err()
        .contains("attributes"));
    fs::write(
        dir.join("nopos.gltf"),
        r#"{"asset":{"version":"2.0"},"meshes":[{"primitives":[{"attributes":{}}]}]}"#,
    )
    .unwrap();
    assert!(load(&dir.join("nopos.gltf"))
        .unwrap_err()
        .contains("POSITION"));

    let empty_pos = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
      "accessors": [{"bufferView": 0, "componentType": 5126, "count": 0, "type": "VEC3"}],
      "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 0}],
      "buffers": [{"byteLength": 0, "uri": "empty.bin"}]
    }"#;
    fs::write(dir.join("empty.bin"), b"").unwrap();
    fs::write(dir.join("empty.gltf"), empty_pos).unwrap();
    assert!(load(&dir.join("empty.gltf")).unwrap_err().contains("empty"));

    let bad_ct = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
      "accessors": [{"bufferView": 0, "componentType": 5123, "count": 3, "type": "VEC3"}],
      "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 6}],
      "buffers": [{"byteLength": 6, "uri": "ct.bin"}]
    }"#;
    fs::write(dir.join("ct.bin"), [0u8; 6]).unwrap();
    fs::write(dir.join("ct.gltf"), bad_ct).unwrap();
    assert!(load(&dir.join("ct.gltf")).unwrap_err().contains("float"));

    let bad_idx = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5122, "count": 3, "type": "SCALAR"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 0, "byteLength": 3}
      ],
      "buffers": [{"byteLength": 36, "uri": "seq.bin"}]
    }"#;
    fs::write(dir.join("badidx.gltf"), bad_idx).unwrap();
    assert!(load(&dir.join("badidx.gltf"))
        .unwrap_err()
        .contains("index"));

    let short_idx = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5121, "count": 8, "type": "SCALAR"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 36, "byteLength": 2}
      ],
      "buffers": [{"byteLength": 38, "uri": "short.bin"}]
    }"#;
    fs::write(
        dir.join("short.bin"),
        pack_f32(&[0.0, 0.8, 0.0, -0.8, -0.7, 0.0, 0.8, -0.7, 0.0]),
    )
    .unwrap();
    fs::write(dir.join("short.gltf"), short_idx).unwrap();
    assert!(load(&dir.join("short.gltf")).unwrap_err().contains("u8"));

    let miss_view = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
      "accessors": [{"componentType": 5126, "count": 1, "type": "VEC3"}],
      "bufferViews": [],
      "buffers": [{"byteLength": 0, "uri": "empty.bin"}]
    }"#;
    fs::write(dir.join("missview.gltf"), miss_view).unwrap();
    assert!(load(&dir.join("missview.gltf")).is_err());

    let overrun = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
      "accessors": [{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}],
      "bufferViews": [{"buffer": 0, "byteOffset": 80, "byteLength": 36}],
      "buffers": [{"byteLength": 10, "uri": "empty.bin"}]
    }"#;
    fs::write(dir.join("over.gltf"), overrun).unwrap();
    assert!(load(&dir.join("over.gltf"))
        .unwrap_err()
        .contains("overrun"));

    let unnamed = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "animations": [{"channels": [{"sampler": 0, "target": {"path": "rotation"}}], "samplers": [{"input": 2, "output": 3}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5121, "count": 3, "type": "SCALAR"},
        {"bufferView": 2, "componentType": 5126, "count": 1, "type": "SCALAR"},
        {"bufferView": 3, "componentType": 5126, "count": 1, "type": "VEC4"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 72, "byteLength": 3},
        {"buffer": 0, "byteOffset": 75, "byteLength": 4},
        {"buffer": 0, "byteOffset": 87, "byteLength": 16}
      ],
      "buffers": [{"byteLength": 171, "uri": "tri.bin"}]
    }"#;
    fs::write(dir.join("tri.bin"), triangle_bin()).unwrap();
    fs::write(dir.join("unnamed.gltf"), unnamed).unwrap();
    let um = load(&dir.join("unnamed.gltf")).unwrap();
    assert!(clip_names(&um).iter().any(|n| n.starts_with("clip")));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn clips_sample_edges_and_synthetic_frames() {
    let empty = Clip {
        name: "x".into(),
        times: vec![],
        rot: vec![],
        trans: vec![],
    };
    let (q, p) = sample_clip(&empty, 1.0, true);
    assert_eq!(q, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(p, [0.0, 0.0, 0.0]);

    let one = Clip {
        name: "one".into(),
        times: vec![0.0],
        rot: vec![[0.0, 0.0, 0.0, 1.0]],
        trans: vec![[1.0, 0.0, 0.0]],
    };
    assert_eq!(sample_clip(&one, 4.0, true).1[0], 1.0);

    let two = Clip {
        name: "two".into(),
        times: vec![0.0, 1.0],
        rot: vec![[0.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 0.0]],
        trans: vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
    };
    let mid = sample_clip(&two, 0.5, false);
    assert!(mid.1[1] > 0.9 && mid.1[1] < 1.1);
    let before = sample_clip(&two, -1.0, false);
    assert_eq!(before.1[1], 0.0);
    let after = sample_clip(&two, 9.0, false);
    assert_eq!(after.1[1], 2.0);
    let looped = sample_clip(&two, 1.5, true);
    assert!(looped.1[1] >= 0.0);
    let same_t = Clip {
        name: "same".into(),
        times: vec![1.0, 1.0],
        rot: vec![[0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
        trans: vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
    };
    let _ = sample_clip(&same_t, 1.0, false);
    let flip = Clip {
        name: "flip".into(),
        times: vec![0.0, 1.0],
        rot: vec![[0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, -1.0]],
        trans: vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
    };
    let _ = sample_clip(&flip, 0.5, false);
    let zeroq = Clip {
        name: "z".into(),
        times: vec![0.0, 1.0],
        rot: vec![[0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]],
        trans: vec![],
    };
    assert_eq!(sample_clip(&zeroq, 0.5, false).0, [0.0, 0.0, 0.0, 1.0]);

    let dir = tmp("syn");
    let gltf = write_triangle(&dir, "plain");
    let raw = fs::read_to_string(&gltf)
        .unwrap()
        .replace("\"animations\"", "\"nope\"");
    fs::write(&gltf, raw).unwrap();
    let mesh = load(&gltf).unwrap();
    assert!(clip_names(&mesh).is_empty());
    let (idle, action) = overlay_frames(&mesh, Some("idle"), Some("action"), 24, 24).unwrap();
    assert_eq!(idle.len(), 2);
    assert_eq!(action.len(), 4);
    assert!(opaque(&idle[0].1));
    let tiny = raster(&mesh, 0, 0, 0.0, None, 0.0);
    assert_eq!(tiny.len(), 4);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn raster_skips_bad_indices_and_sidecars_scan() {
    let dir = tmp("side");
    let gltf = write_triangle(&dir, "blob");
    fs::write(dir.join("blob.png"), b"not-a-real-png").unwrap();
    let sides = gltf_sidecars(&gltf);
    assert!(sides.iter().any(|p| p.ends_with("blob.bin")));
    fs::write(
        dir.join("img.gltf"),
        r#"{"buffers":[{"uri":"data:foo"}],"images":[{"uri":"http://x/a.png"},{"uri":"blob.png"},{"uri":""}]}"#,
    )
    .unwrap();
    let imgs = gltf_sidecars(&dir.join("img.gltf"));
    assert!(imgs.iter().any(|p| p.ends_with("blob.png")));
    fs::write(dir.join("broken.gltf"), "not-json").unwrap();
    fs::write(dir.join("broken.bin"), b"xx").unwrap();
    let sib = gltf_sidecars(&dir.join("broken.gltf"));
    assert!(sib.iter().any(|p| p.ends_with("broken.bin")));

    let mut mesh = load(&gltf).unwrap();
    mesh.idx = vec![0, 1, 99, 0, 1, 2];
    let px = raster_pose(&mesh, 16, 16, 0.2, [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
    assert!(opaque(&px));
    mesh.pos = vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
    mesh.center = [0.0, 0.0, 0.0];
    mesh.span = 0.0;
    mesh.idx = vec![0, 1, 2];
    let flat = raster_pose(&mesh, 8, 8, 0.0, [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
    assert_eq!(flat.len(), 256);
    let miss = raster_file(&dir.join("nope.gltf"), 4, 4);
    assert_eq!(miss, vec![0; 64]);

    let mut zmesh = load(&gltf).unwrap();
    zmesh.nrm = vec![[0.0, 0.0, 0.0]; zmesh.pos.len()];
    let _ = raster_pose(&zmesh, 12, 12, 0.1, [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
    fs::write(
        dir.join("nouri-img.gltf"),
        r#"{"images":[{},{"uri":"missing.png"}]}"#,
    )
    .unwrap();
    let _ = gltf_sidecars(&dir.join("nouri-img.gltf"));

    let bad_rot = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "animations": [{"name":"idle","channels": [
        {"sampler": 0, "target": {"path": "rotation"}},
        {"sampler": 1, "target": {"path": "translation"}}
      ], "samplers": [
        {"input": 99, "output": 4},
        {"input": 2, "output": 99}
      ]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5121, "count": 3, "type": "SCALAR"},
        {"bufferView": 3, "componentType": 5123, "count": 1, "type": "SCALAR"},
        {"bufferView": 3, "componentType": 5123, "count": 1, "type": "VEC4"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 72, "byteLength": 3},
        {"buffer": 0, "byteOffset": 75, "byteLength": 12},
        {"buffer": 0, "byteOffset": 72, "byteLength": 2}
      ],
      "buffers": [{"byteLength": 171, "uri": "blob.bin"}]
    }"#;
    fs::write(dir.join("badrot.gltf"), bad_rot).unwrap();
    let br = load(&dir.join("badrot.gltf")).unwrap();
    assert!(br.clips.contains_key("idle"));
    let _ = overlay_frames(&br, None, None, 16, 16);

    let short16 = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5123, "count": 8, "type": "SCALAR"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 36, "byteLength": 2}
      ],
      "buffers": [{"byteLength": 42, "uri": "u16.bin"}]
    }"#;
    fs::write(
        dir.join("u16.bin"),
        pack_f32(&[0.0, 0.8, 0.0, -0.8, -0.7, 0.0, 0.8, -0.7, 0.0]),
    )
    .unwrap();
    fs::write(dir.join("s16.gltf"), short16).unwrap();
    assert!(load(&dir.join("s16.gltf")).unwrap_err().contains("u16"));

    let short32 = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5125, "count": 8, "type": "SCALAR"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 36, "byteLength": 4}
      ],
      "buffers": [{"byteLength": 40, "uri": "u16.bin"}]
    }"#;
    fs::write(dir.join("s32.gltf"), short32).unwrap();
    assert!(load(&dir.join("s32.gltf")).unwrap_err().contains("u32"));

    let mut bad_json_glb = b"glTF".to_vec();
    bad_json_glb.extend(2u32.to_le_bytes());
    let payload = b"{";
    let total = 12 + 8 + payload.len();
    bad_json_glb.extend((total as u32).to_le_bytes());
    bad_json_glb.extend((payload.len() as u32).to_le_bytes());
    bad_json_glb.extend(0x4E4F534Au32.to_le_bytes());
    bad_json_glb.extend(payload);
    fs::write(dir.join("badjson.glb"), bad_json_glb).unwrap();
    assert!(load(&dir.join("badjson.glb")).unwrap_err().contains("json"));

    let v4 = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "animations": [{"name":"spin","channels":[{"sampler":0,"target":{"path":"rotation"}}],"samplers":[{"input":2,"output":3}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5121, "count": 3, "type": "SCALAR"},
        {"bufferView": 2, "componentType": 5126, "count": 1, "type": "SCALAR"},
        {"bufferView": 3, "componentType": 5123, "count": 1, "type": "VEC4"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 72, "byteLength": 3},
        {"buffer": 0, "byteOffset": 75, "byteLength": 4},
        {"buffer": 0, "byteOffset": 72, "byteLength": 2}
      ],
      "buffers": [{"byteLength": 171, "uri": "blob.bin"}]
    }"#;
    fs::write(dir.join("v4.gltf"), v4).unwrap();
    let vm = load(&dir.join("v4.gltf")).unwrap();
    assert!(vm.clips.get("spin").is_some());

    let times_bad = v4.replace(
        r#"{"bufferView": 2, "componentType": 5126, "count": 1, "type": "SCALAR"}"#,
        r#"{"bufferView": 2, "componentType": 5123, "count": 1, "type": "SCALAR"}"#,
    );
    fs::write(dir.join("tbad.gltf"), times_bad).unwrap();
    let _ = load(&dir.join("tbad.gltf"));

    let miss_bv = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
      "accessors": [{"bufferView": 9, "componentType": 5126, "count": 1, "type": "VEC3"}],
      "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 12}],
      "buffers": [{"byteLength": 12, "uri": "blob.bin"}]
    }"#;
    fs::write(dir.join("missbv.gltf"), miss_bv).unwrap();
    assert!(load(&dir.join("missbv.gltf"))
        .unwrap_err()
        .contains("bufferView"));

    let bad_idx_view = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 9, "componentType": 5123, "count": 3, "type": "SCALAR"}
      ],
      "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}],
      "buffers": [{"byteLength": 171, "uri": "blob.bin"}]
    }"#;
    fs::write(dir.join("badidxv.gltf"), bad_idx_view).unwrap();
    assert!(load(&dir.join("badidxv.gltf")).is_err());

    let empty_anim = triangle_gltf(
        "blob.bin",
        r#","animations":[{"name":"none"},{"name":"hop","channels":[{"sampler":0,"target":{"path":"rotation"}},{"sampler":0,"target":{"path":"scale"}}],"samplers":[{"input":3,"output":4}]}]"#,
    );
    // replace existing animations key by using a dedicated file
    let ea = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2, "material": 0}]}],
      "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [0.2, 0.7, 0.3, 1]}}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.9,-0.8,0], "max": [0.9,0.9,0]},
        {"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 2, "componentType": 5121, "count": 3, "type": "SCALAR"},
        {"bufferView": 3, "componentType": 5126, "count": 3, "type": "SCALAR", "min": [0], "max": [0.8]},
        {"bufferView": 4, "componentType": 5126, "count": 3, "type": "VEC4"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 36, "byteLength": 36},
        {"buffer": 0, "byteOffset": 72, "byteLength": 3},
        {"buffer": 0, "byteOffset": 75, "byteLength": 12},
        {"buffer": 0, "byteOffset": 87, "byteLength": 48}
      ],
      "animations": [
        {"name":"none"},
        {"name":"hop","channels":[{"sampler":0,"target":{"path":"rotation"}},{"sampler":0,"target":{"path":"scale"}}],"samplers":[{"input":3,"output":4}]}
      ],
      "buffers": [{"byteLength": 171, "uri": "blob.bin"}]
    }"#;
    fs::write(dir.join("emptyanim.gltf"), ea).unwrap();
    let em = load(&dir.join("emptyanim.gltf")).unwrap();
    assert!(em.clips.contains_key("none"));
    assert!(em.clips.contains_key("hop"));
    let _ = empty_anim;

    fs::write(dir.join("nobin.gltf"), triangle_gltf("gone.bin", "")).unwrap();
    assert!(load(&dir.join("nobin.gltf")).unwrap_err().contains("bin"));

    let short_v4 = r#"{
      "asset": {"version":"2.0"},
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
      "animations": [{"name":"spin","channels":[{"sampler":0,"target":{"path":"rotation"}}],"samplers":[{"input":2,"output":3}]}],
      "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
        {"bufferView": 1, "componentType": 5121, "count": 3, "type": "SCALAR"},
        {"bufferView": 2, "componentType": 5126, "count": 1, "type": "SCALAR"},
        {"bufferView": 3, "componentType": 5126, "count": 8, "type": "VEC4"}
      ],
      "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 36},
        {"buffer": 0, "byteOffset": 72, "byteLength": 3},
        {"buffer": 0, "byteOffset": 75, "byteLength": 4},
        {"buffer": 0, "byteOffset": 87, "byteLength": 8}
      ],
      "buffers": [{"byteLength": 171, "uri": "blob.bin"}]
    }"#;
    fs::write(dir.join("shortv4.gltf"), short_v4).unwrap();
    let _ = load(&dir.join("shortv4.gltf"));

    let cap =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pack/models/capybara/capybara.gltf");
    let capm = load(&cap).unwrap();
    let _ = overlay_frames(&capm, None, None, 24, 24).unwrap();
    let zclip = Clip {
        name: "idle".into(),
        times: vec![0.0],
        rot: vec![],
        trans: vec![],
    };
    let mut zc = capm.clone();
    zc.clips.insert("idle".into(), zclip);
    let _ = overlay_frames(&zc, Some("idle"), Some("missing"), 16, 16).unwrap();
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn overlay_frames_reject_empty_mesh() {
    let mesh = Mesh {
        pos: vec![[0.0, 0.0, 0.0]],
        nrm: vec![[0.0, 1.0, 0.0]],
        idx: vec![],
        color: [1.0, 0.0, 0.0],
        center: [0.0, 0.0, 0.0],
        span: 1.0,
        clips: Default::default(),
    };
    let err = overlay_frames(&mesh, None, None, 8, 8).unwrap_err();
    assert!(err.contains("empty") || err.contains("no overlay"), "{err}");
}
