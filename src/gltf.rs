//! Minimal glTF 2.0 mesh + clip loader and a Lambert software rasterizer.
//! Overlay is a layered bitmap; this turns a 3D pack file into idle/action frames.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

type FrameSeq = Vec<(String, Vec<u8>)>;

#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub times: Vec<f32>,
    pub rot: Vec<[f32; 4]>,
    pub trans: Vec<[f32; 3]>,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub idx: Vec<u32>,
    pub color: [f32; 3],
    pub center: [f32; 3],
    pub span: f32,
    pub clips: BTreeMap<String, Clip>,
}

const Q_ID: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const T_Z: [f32; 3] = [0.0, 0.0, 0.0];

pub fn load(path: &Path) -> Result<Mesh, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("gltf {}: {e}", path.display()))?;
    let (doc, bin) = if path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .eq_ignore_ascii_case("glb")
    {
        parse_glb(&bytes)?
    } else {
        let doc: Value = serde_json::from_slice(&bytes).map_err(|e| format!("gltf json: {e}"))?;
        let bin = load_gltf_bin(path, &doc)?;
        (doc, bin)
    };
    mesh_from_doc(&doc, &bin)
}

pub fn raster_file(path: &Path, w: u32, h: u32) -> Vec<u8> {
    match load(path) {
        Ok(mesh) => raster(&mesh, w, h, 0.35, None, 0.0),
        Err(_) => vec![0; (w.max(1) as usize) * (h.max(1) as usize) * 4],
    }
}

pub fn clip_names(mesh: &Mesh) -> Vec<String> {
    mesh.clips.keys().cloned().collect()
}

pub fn raster(mesh: &Mesh, w: u32, h: u32, yaw: f32, clip: Option<&str>, t: f32) -> Vec<u8> {
    let (q, p) = match clip.and_then(|n| mesh.clips.get(n)) {
        Some(c) => sample_clip(c, t, true),
        None => (Q_ID, T_Z),
    };
    raster_pose(mesh, w, h, yaw, q, p)
}

pub fn overlay_frames(
    mesh: &Mesh,
    idle_clip: Option<&str>,
    action_clip: Option<&str>,
    w: u32,
    h: u32,
) -> Result<(FrameSeq, FrameSeq), String> {
    let idle_name = pick_clip(mesh, idle_clip, &["idle", "sway"]);
    let action_name = pick_clip(mesh, action_clip, &["headbutt-cycle", "action", "hit"]);
    let idle = sample_named(mesh, idle_name.as_deref(), 2, w, h, true);
    let action = sample_named(mesh, action_name.as_deref(), 4, w, h, false);
    if !opaque(&idle[0].1) && !opaque(&action[0].1) {
        return Err("3d mesh raster is empty".into());
    }
    Ok((idle, action))
}

fn pick_clip(mesh: &Mesh, want: Option<&str>, aliases: &[&str]) -> Option<String> {
    if let Some(n) = want {
        if mesh.clips.contains_key(n) {
            return Some(n.to_string());
        }
    }
    for a in aliases {
        if mesh.clips.contains_key(*a) {
            return Some((*a).to_string());
        }
    }
    mesh.clips.keys().next().cloned()
}

fn sample_named(
    mesh: &Mesh,
    name: Option<&str>,
    n: usize,
    w: u32,
    h: u32,
    gentle: bool,
) -> Vec<(String, Vec<u8>)> {
    match name.and_then(|n| mesh.clips.get(n)) {
        Some(clip) => {
            let dur = clip.times.last().copied().unwrap_or(0.0);
            (0..n)
                .map(|i| {
                    let t = dur * (i as f32 + 0.5) / (n as f32).max(1.0);
                    let yaw = if gentle { 0.12 } else { 0.0 };
                    (
                        format!("{}_{i:02}", clip.name),
                        raster(mesh, w, h, yaw, Some(&clip.name), t),
                    )
                })
                .collect()
        }
        None => synthetic_turn(mesh, n, w, h),
    }
}

fn synthetic_turn(mesh: &Mesh, n: usize, w: u32, h: u32) -> Vec<(String, Vec<u8>)> {
    (0..n)
        .map(|i| {
            let yaw = 0.35 + (i as f32) * 0.55;
            (format!("turn_{i:02}"), raster(mesh, w, h, yaw, None, 0.0))
        })
        .collect()
}

fn opaque(px: &[u8]) -> bool {
    px.chunks(4).any(|p| p[3] > 24)
}

pub fn sample_clip(clip: &Clip, t: f32, looped: bool) -> ([f32; 4], [f32; 3]) {
    if clip.times.is_empty() {
        return (Q_ID, T_Z);
    }
    let last = *clip.times.last().unwrap_or(&0.0);
    let t = if looped && last > 0.0 {
        t.rem_euclid(last)
    } else if last > 0.0 {
        t.clamp(0.0, last)
    } else {
        0.0
    };
    if clip.times.len() == 1 || t <= clip.times[0] {
        return (rot_at(clip, 0), trans_at(clip, 0));
    }
    if t >= last {
        let i = clip.times.len() - 1;
        return (rot_at(clip, i), trans_at(clip, i));
    }
    let mut i = 0;
    while i + 1 < clip.times.len() && clip.times[i + 1] < t {
        i += 1;
    }
    let t0 = clip.times[i];
    let t1 = clip.times[i + 1];
    let u = (t - t0) / (t1 - t0 + 1e-20);
    (
        nlerp(rot_at(clip, i), rot_at(clip, i + 1), u),
        lerp3(trans_at(clip, i), trans_at(clip, i + 1), u),
    )
}

fn rot_at(clip: &Clip, i: usize) -> [f32; 4] {
    clip.rot.get(i).copied().unwrap_or(Q_ID)
}

fn trans_at(clip: &Clip, i: usize) -> [f32; 3] {
    clip.trans.get(i).copied().unwrap_or(T_Z)
}

fn nlerp(a: [f32; 4], mut b: [f32; 4], t: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }
    let s = 1.0 - t;
    let mut q = [
        s * a[0] + t * b[0],
        s * a[1] + t * b[1],
        s * a[2] + t * b[2],
        s * a[3] + t * b[3],
    ];
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if n < 1e-8 {
        return Q_ID;
    }
    q[0] /= n;
    q[1] /= n;
    q[2] /= n;
    q[3] /= n;
    let _ = dot;
    q
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn qrot(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let u = [q[0], q[1], q[2]];
    let s = q[3];
    let uv = cross(u, v);
    let uuv = cross(u, [uv[0] + s * v[0], uv[1] + s * v[1], uv[2] + s * v[2]]);
    [
        v[0] + 2.0 * uuv[0],
        v[1] + 2.0 * uuv[1],
        v[2] + 2.0 * uuv[2],
    ]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm3(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if n < 1e-8 {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / n, v[1] / n, v[2] / n]
    }
}

pub fn raster_pose(mesh: &Mesh, w: u32, h: u32, yaw: f32, q: [f32; 4], trans: [f32; 3]) -> Vec<u8> {
    let w = w.max(1) as usize;
    let h = h.max(1) as usize;
    let mut rgba = vec![0u8; w * h * 4];
    let mut zbuf = vec![f32::INFINITY; w * h];
    let c = yaw.cos();
    let s = yaw.sin();
    let scale = if mesh.span > 1e-6 {
        1.55 / mesh.span
    } else {
        1.0
    };
    let light = norm3([0.35, 0.8, 0.5]);
    let mut xf = vec![[0.0f32; 3]; mesh.pos.len()];
    let mut xn = vec![[0.0f32; 3]; mesh.pos.len()];
    for i in 0..mesh.pos.len() {
        let mut p = qrot(q, mesh.pos[i]);
        p = [p[0] + trans[0], p[1] + trans[1], p[2] + trans[2]];
        p = [
            (p[0] - mesh.center[0]) * scale,
            (p[1] - mesh.center[1]) * scale,
            (p[2] - mesh.center[2]) * scale,
        ];
        xf[i] = [c * p[0] + s * p[2], p[1], -s * p[0] + c * p[2]];
        let n = qrot(q, mesh.nrm.get(i).copied().unwrap_or([0.0, 1.0, 0.0]));
        xn[i] = [c * n[0] + s * n[2], n[1], -s * n[0] + c * n[2]];
    }
    let wf = w as f32;
    let hf = h as f32;
    let mut i = 0;
    while i + 2 < mesh.idx.len() {
        let a = mesh.idx[i] as usize;
        let b = mesh.idx[i + 1] as usize;
        let c = mesh.idx[i + 2] as usize;
        i += 3;
        if a >= xf.len() || b >= xf.len() || c >= xf.len() {
            continue;
        }
        let pa = xf[a];
        let pb = xf[b];
        let pc = xf[c];
        let sa = to_screen(pa, wf, hf);
        let sb = to_screen(pb, wf, hf);
        let sc = to_screen(pc, wf, hf);
        let area = edge(sa, sb, sc);
        if area.abs() < 1e-5 {
            continue;
        }
        let minx = sa[0].min(sb[0]).min(sc[0]).floor().max(0.0) as usize;
        let maxx = sa[0].max(sb[0]).max(sc[0]).ceil().min(wf - 1.0) as usize;
        let miny = sa[1].min(sb[1]).min(sc[1]).floor().max(0.0) as usize;
        let maxy = sa[1].max(sb[1]).max(sc[1]).ceil().min(hf - 1.0) as usize;
        let na = norm3(xn[a]);
        let nb = norm3(xn[b]);
        let nc = norm3(xn[c]);
        let inv = 1.0 / area;
        for y in miny..=maxy {
            for x in minx..=maxx {
                let p = [x as f32 + 0.5, y as f32 + 0.5];
                let w0 = edge(sb, sc, p) * inv;
                let w1 = edge(sc, sa, p) * inv;
                let w2 = edge(sa, sb, p) * inv;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * pa[2] + w1 * pb[2] + w2 * pc[2];
                let pix = y * w + x;
                if z >= zbuf[pix] {
                    continue;
                }
                zbuf[pix] = z;
                let n = norm3([
                    w0 * na[0] + w1 * nb[0] + w2 * nc[0],
                    w0 * na[1] + w1 * nb[1] + w2 * nc[1],
                    w0 * na[2] + w1 * nb[2] + w2 * nc[2],
                ]);
                let d = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).abs();
                let l = 0.38 + 0.62 * d;
                let o = pix * 4;
                rgba[o] = (mesh.color[0] * l * 255.0).clamp(0.0, 255.0) as u8;
                rgba[o + 1] = (mesh.color[1] * l * 255.0).clamp(0.0, 255.0) as u8;
                rgba[o + 2] = (mesh.color[2] * l * 255.0).clamp(0.0, 255.0) as u8;
                rgba[o + 3] = 255;
            }
        }
    }
    rgba
}

fn to_screen(p: [f32; 3], w: f32, h: f32) -> [f32; 2] {
    [
        (p[0] * 0.5 + 0.5) * (w - 1.0).max(1.0),
        ((-p[1] * 0.74) * 0.5 + 0.5) * (h - 1.0).max(1.0),
    ]
}

fn edge(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])
}

fn parse_glb(bytes: &[u8]) -> Result<(Value, Vec<u8>), String> {
    if bytes.len() < 12 {
        return Err("glb too small".into());
    }
    if &bytes[0..4] != b"glTF" {
        return Err("glb magic".into());
    }
    let version = u32le(bytes, 4);
    if version != 2 {
        return Err("glb version".into());
    }
    let mut off = 12usize;
    let mut json = None;
    let mut bin = Vec::new();
    while off + 8 <= bytes.len() {
        let clen = u32le(bytes, off) as usize;
        let ctype = u32le(bytes, off + 4);
        off += 8;
        if off + clen > bytes.len() {
            return Err("glb chunk".into());
        }
        let chunk = &bytes[off..off + clen];
        off += clen;
        if ctype == 0x4E4F534A {
            json = Some(serde_json::from_slice(chunk).map_err(|e| format!("glb json: {e}"))?);
        } else if ctype == 0x004E4942 {
            bin = chunk.to_vec();
        }
    }
    let doc = json.ok_or_else(|| "glb missing json".to_string())?;
    Ok((doc, bin))
}

fn load_gltf_bin(path: &Path, doc: &Value) -> Result<Vec<u8>, String> {
    let uri = doc
        .get("buffers")
        .and_then(|b| b.get(0))
        .and_then(|b| b.get("uri"))
        .and_then(|u| u.as_str())
        .unwrap_or("");
    if uri.is_empty() {
        return Ok(Vec::new());
    }
    if uri.starts_with("data:") {
        return Err("data uri buffers unsupported".into());
    }
    let mut bin_path = path.to_path_buf();
    bin_path.set_file_name(uri.replace('\\', "/"));
    std::fs::read(&bin_path).map_err(|e| format!("bin {}: {e}", bin_path.display()))
}

fn mesh_from_doc(doc: &Value, bin: &[u8]) -> Result<Mesh, String> {
    let mesh0 = doc
        .get("meshes")
        .and_then(|m| m.get(0))
        .ok_or_else(|| "gltf has no meshes".to_string())?;
    let prim = mesh0
        .get("primitives")
        .and_then(|p| p.get(0))
        .ok_or_else(|| "gltf has no primitives".to_string())?;
    let attrs = prim
        .get("attributes")
        .ok_or_else(|| "gltf missing attributes".to_string())?;
    let pos_i = attrs
        .get("POSITION")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "gltf missing POSITION".to_string())? as usize;
    let views = doc
        .get("bufferViews")
        .cloned()
        .unwrap_or(Value::Array(vec![]));
    let accs = doc
        .get("accessors")
        .cloned()
        .unwrap_or(Value::Array(vec![]));
    let pos = read_vec3(bin, &accs, &views, pos_i)?;
    if pos.is_empty() {
        return Err("gltf empty POSITION".into());
    }
    let nrm = match attrs.get("NORMAL").and_then(|v| v.as_u64()) {
        Some(i) => read_vec3(bin, &accs, &views, i as usize).unwrap_or_default(),
        None => Vec::new(),
    };
    let nrm = if nrm.len() == pos.len() {
        nrm
    } else {
        vec![[0.0, 1.0, 0.0]; pos.len()]
    };
    let idx = match prim.get("indices").and_then(|v| v.as_u64()) {
        Some(i) => read_indices(bin, &accs, &views, i as usize)?,
        None => (0..pos.len() as u32).collect(),
    };
    let color = doc
        .pointer("/materials/0/pbrMetallicRoughness/baseColorFactor")
        .and_then(|v| v.as_array())
        .map(|a| {
            [
                a.first().and_then(|x| x.as_f64()).unwrap_or(0.62) as f32,
                a.get(1).and_then(|x| x.as_f64()).unwrap_or(0.38) as f32,
                a.get(2).and_then(|x| x.as_f64()).unwrap_or(0.22) as f32,
            ]
        })
        .unwrap_or([0.62, 0.38, 0.22]);
    let (min, max) = bounds(&pos);
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let span = (max[0] - min[0])
        .max(max[1] - min[1])
        .max(max[2] - min[2])
        .max(0.5);
    let clips = load_clips(doc, bin, &accs, &views);
    Ok(Mesh {
        pos,
        nrm,
        idx,
        color,
        center,
        span,
        clips,
    })
}

fn bounds(pos: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for p in pos {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    (min, max)
}

#[inline(never)]
fn load_clips(doc: &Value, bin: &[u8], accs: &Value, views: &Value) -> BTreeMap<String, Clip> {
    let mut out = BTreeMap::new();
    let Some(anims) = doc.get("animations").and_then(|a| a.as_array()) else {
        return out;
    };
    for (ai, anim) in anims.iter().enumerate() {
        let name = anim
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("clip{ai}"));
        let mut times = Vec::new();
        let mut rot = Vec::new();
        let mut trans = Vec::new();
        let channels = anim.get("channels").and_then(|c| c.as_array());
        let samplers = anim.get("samplers").and_then(|s| s.as_array());
        if let (Some(channels), Some(samplers)) = (channels, samplers) {
            for ch in channels {
                let si = ch.get("sampler").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let path = ch
                    .pointer("/target/path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let Some(samp) = samplers.get(si) else {
                    continue;
                };
                let ii = samp.get("input").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let oi = samp.get("output").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                if times.is_empty() {
                    times = read_scalars(bin, accs, views, ii).unwrap_or_default();
                }
                match path {
                    "rotation" => rot = read_vec4(bin, accs, views, oi).unwrap_or_default(),
                    "translation" => trans = read_vec3(bin, accs, views, oi).unwrap_or_default(),
                    other => {
                        let _ = other.len();
                    }
                }
            }
        } else {
            let _ = name.len();
        }
        out.insert(
            name.clone(),
            Clip {
                name,
                times,
                rot,
                trans,
            },
        );
    }
    out
}

fn acc(accs: &Value, i: usize) -> Result<&Value, String> {
    accs.get(i).ok_or_else(|| format!("accessor {i}"))
}

fn view_slice<'a>(
    bin: &'a [u8],
    accs: &'a Value,
    views: &'a Value,
    i: usize,
) -> Result<(&'a [u8], &'a Value), String> {
    let a = acc(accs, i)?;
    let vi = a
        .get("bufferView")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("accessor {i} bufferView"))? as usize;
    let v = views.get(vi).ok_or_else(|| format!("bufferView {vi}"))?;
    let off = v.get("byteOffset").and_then(|x| x.as_u64()).unwrap_or(0) as usize
        + a.get("byteOffset").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
    let blen = v.get("byteLength").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
    if off > bin.len() {
        return Err("buffer overrun".into());
    }
    let end = (off + blen).min(bin.len());
    Ok((&bin[off..end], a))
}

fn read_f32_run(slice: &[u8], n: usize, comps: usize) -> Result<Vec<f32>, String> {
    let need = n.saturating_mul(comps).saturating_mul(4);
    if slice.len() < need {
        return Err("float accessor short".into());
    }
    let mut out = Vec::with_capacity(n * comps);
    for i in 0..(n * comps) {
        out.push(f32::from_le_bytes(bytes4(slice, i * 4)));
    }
    Ok(out)
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(bytes4(b, o))
}

fn bytes4(b: &[u8], o: usize) -> [u8; 4] {
    [
        b.get(o).copied().unwrap_or(0),
        b.get(o + 1).copied().unwrap_or(0),
        b.get(o + 2).copied().unwrap_or(0),
        b.get(o + 3).copied().unwrap_or(0),
    ]
}

fn bytes2(b: &[u8], o: usize) -> [u8; 2] {
    [
        b.get(o).copied().unwrap_or(0),
        b.get(o + 1).copied().unwrap_or(0),
    ]
}

fn read_vec3(bin: &[u8], accs: &Value, views: &Value, i: usize) -> Result<Vec<[f32; 3]>, String> {
    let (slice, a) = view_slice(bin, accs, views, i)?;
    let n = a.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let ct = a
        .get("componentType")
        .and_then(|v| v.as_u64())
        .unwrap_or(5126);
    if ct != 5126 {
        return Err("vec3 must be float".into());
    }
    let flat = read_f32_run(slice, n, 3)?;
    Ok(flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

#[inline(never)]
fn read_vec4(bin: &[u8], accs: &Value, views: &Value, i: usize) -> Result<Vec<[f32; 4]>, String> {
    let (slice, a) = view_slice(bin, accs, views, i)?;
    let n = a.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let ct = a
        .get("componentType")
        .and_then(|v| v.as_u64())
        .unwrap_or(5126);
    if ct != 5126 {
        return Err("vec4 must be float".into());
    }
    let flat = read_f32_run(slice, n, 4)?;
    Ok(flat
        .chunks_exact(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect())
}

#[inline(never)]
fn read_scalars(bin: &[u8], accs: &Value, views: &Value, i: usize) -> Result<Vec<f32>, String> {
    let (slice, a) = view_slice(bin, accs, views, i)?;
    let n = a.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let ct = a
        .get("componentType")
        .and_then(|v| v.as_u64())
        .unwrap_or(5126);
    if ct != 5126 {
        return Err("times must be float".into());
    }
    read_f32_run(slice, n, 1)
}

fn read_indices(bin: &[u8], accs: &Value, views: &Value, i: usize) -> Result<Vec<u32>, String> {
    let (slice, a) = view_slice(bin, accs, views, i)?;
    let n = a.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let ct = a
        .get("componentType")
        .and_then(|v| v.as_u64())
        .unwrap_or(5123);
    match ct {
        5121 => {
            if slice.len() < n {
                return Err("u8 indices short".into());
            }
            Ok(slice[..n].iter().map(|b| *b as u32).collect())
        }
        5123 => {
            if slice.len() < n * 2 {
                return Err("u16 indices short".into());
            }
            Ok((0..n)
                .map(|k| u16::from_le_bytes(bytes2(slice, k * 2)) as u32)
                .collect())
        }
        5125 => {
            if slice.len() < n * 4 {
                return Err("u32 indices short".into());
            }
            Ok((0..n).map(|k| u32le(slice, k * 4)).collect())
        }
        _ => Err("index componentType".into()),
    }
}

pub fn gltf_sidecars(src: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let sibling = src.with_extension("bin");
    if sibling.is_file() {
        out.push(sibling);
    }
    let Ok(raw) = std::fs::read_to_string(src) else {
        return out;
    };
    let Ok(doc) = serde_json::from_str::<Value>(&raw) else {
        return out;
    };
    let parent = src.parent().unwrap_or(Path::new("."));
    if let Some(buffers) = doc.get("buffers").and_then(|b| b.as_array()) {
        for b in buffers {
            push_uri(parent, b.get("uri").and_then(|u| u.as_str()), &mut out);
        }
    }
    if let Some(images) = doc.get("images").and_then(|b| b.as_array()) {
        for b in images {
            push_uri(parent, b.get("uri").and_then(|u| u.as_str()), &mut out);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn push_uri(parent: &Path, uri: Option<&str>, out: &mut Vec<PathBuf>) {
    let Some(uri) = uri else {
        return;
    };
    if uri.is_empty() || uri.starts_with("data:") || uri.contains("://") {
        return;
    }
    let p = parent.join(uri.replace('\\', "/"));
    if p.is_file() {
        out.push(p);
    }
}
