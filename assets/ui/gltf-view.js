(function (global) {
  let raf = 0;
  let gen = 0;
  let scene = null;

  function stop() {
    gen += 1;
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
    scene = null;
  }

  function compile(gl, type, src) {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      throw new Error(gl.getShaderInfoLog(s) || 'shader');
    }
    return s;
  }

  function typed(bin, acc, views, comps) {
    const v = views[acc.bufferView];
    const off = (v.byteOffset || 0) + (acc.byteOffset || 0);
    const n = acc.count * comps;
    const bytes = acc.componentType === 5126 ? n * 4 : n * 2;
    const slice = bin.slice(off, off + bytes);
    if (acc.componentType === 5126) return new Float32Array(slice);
    if (acc.componentType === 5123) return new Uint16Array(slice);
    return new Uint32Array(slice);
  }

  function nlerp(a, b, t) {
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    const bb = dot < 0 ? [-b[0], -b[1], -b[2], -b[3]] : b;
    if (dot < 0) dot = -dot;
    const s = 1 - t;
    const q = [s * a[0] + t * bb[0], s * a[1] + t * bb[1], s * a[2] + t * bb[2], s * a[3] + t * bb[3]];
    const n = Math.hypot(q[0], q[1], q[2], q[3]) || 1;
    return [q[0] / n, q[1] / n, q[2] / n, q[3] / n];
  }

  function sampleClip(clip, t, looped) {
    const idQ = [0, 0, 0, 1];
    const idT = [0, 0, 0];
    if (!clip || !clip.times || !clip.times.length) return { q: idQ, p: idT };
    const last = clip.times[clip.times.length - 1];
    if (looped && last > 0) t = ((t % last) + last) % last;
    else if (last > 0) t = Math.min(Math.max(t, 0), last);
    else t = 0;
    if (clip.times.length === 1 || t <= clip.times[0]) {
      return { q: clip.rot[0] || idQ, p: clip.trans[0] || idT };
    }
    if (t >= last) {
      const i = clip.times.length - 1;
      return { q: clip.rot[i] || idQ, p: clip.trans[i] || idT };
    }
    let i = 0;
    while (i + 1 < clip.times.length && clip.times[i + 1] < t) i += 1;
    const t0 = clip.times[i];
    const t1 = clip.times[i + 1];
    const u = Math.abs(t1 - t0) < 1e-8 ? 0 : (t - t0) / (t1 - t0);
    const qa = clip.rot[i] || idQ;
    const qb = clip.rot[i + 1] || idQ;
    const pa = clip.trans[i] || idT;
    const pb = clip.trans[i + 1] || idT;
    return {
      q: nlerp(qa, qb, u),
      p: [pa[0] + (pb[0] - pa[0]) * u, pa[1] + (pb[1] - pa[1]) * u, pa[2] + (pb[2] - pa[2]) * u],
    };
  }

  function readClips(gltf, bin, acc, views) {
    const clips = {};
    (gltf.animations || []).forEach(function (anim, ai) {
      const name = anim.name || ('clip' + ai);
      const times = [];
      const rot = [];
      const trans = [];
      (anim.channels || []).forEach(function (ch) {
        const samp = (anim.samplers || [])[ch.sampler || 0];
        if (!samp) return;
        const path = ch.target && ch.target.path;
        if (!times.length) {
          const ta = acc[samp.input];
          if (ta) typed(bin, ta, views, 1).forEach(function (v) { times.push(v); });
        }
        if (path === 'rotation') {
          const a = acc[samp.output];
          if (!a) return;
          const arr = typed(bin, a, views, 4);
          for (let i = 0; i < arr.length; i += 4) rot.push([arr[i], arr[i + 1], arr[i + 2], arr[i + 3]]);
        }
        if (path === 'translation') {
          const a = acc[samp.output];
          if (!a) return;
          const arr = typed(bin, a, views, 3);
          for (let i = 0; i < arr.length; i += 3) trans.push([arr[i], arr[i + 1], arr[i + 2]]);
        }
      });
      clips[name] = { name: name, times: times, rot: rot, trans: trans };
    });
    return clips;
  }

  function joinUrl(url, name) {
    if (!name) return url;
    if (/^https?:/i.test(name) || name.indexOf('data:') === 0) return name;
    return url.replace(/[^/]+$/, encodeURIComponent(name));
  }

  async function loadGltf(url) {
    const res = await fetch(url);
    if (!res.ok) throw new Error('gltf ' + res.status);
    const buf = await res.arrayBuffer();
    const head = new Uint8Array(buf, 0, 4);
    const isGlb = head[0] === 0x67 && head[1] === 0x6c && head[2] === 0x54 && head[3] === 0x46;
    if (!isGlb) {
      const gltf = JSON.parse(new TextDecoder().decode(buf));
      const binName = (gltf.buffers && gltf.buffers[0] && gltf.buffers[0].uri) || '';
      let bin = new ArrayBuffer(0);
      if (binName && binName.indexOf('data:') !== 0) {
        const br = await fetch(joinUrl(url, binName));
        if (!br.ok) throw new Error('bin ' + br.status);
        bin = await br.arrayBuffer();
      }
      return { gltf: gltf, bin: bin };
    }
    const dv = new DataView(buf);
    let off = 12;
    let json = null;
    let bin = new ArrayBuffer(0);
    while (off + 8 <= buf.byteLength) {
      const len = dv.getUint32(off, true);
      const typ = dv.getUint32(off + 4, true);
      const slice = buf.slice(off + 8, off + 8 + len);
      if (typ === 0x4e4f534a) json = JSON.parse(new TextDecoder().decode(slice));
      if (typ === 0x004e4942) bin = slice;
      off += 8 + len;
    }
    if (!json) throw new Error('glb json');
    return { gltf: json, bin: bin };
  }

  function play(which, names) {
    if (!scene) return;
    scene.mode = which || 'still';
    scene.t0 = performance.now();
    if (names && names.idle) scene.idleName = names.idle;
    if (names && names.action) scene.actionName = names.action;
  }

  function glContext(canvas) {
    const opts = { alpha: true, antialias: true, preserveDrawingBuffer: true, premultipliedAlpha: true };
    return canvas.getContext('webgl', opts) || canvas.getContext('experimental-webgl', opts);
  }

  async function mount(canvas, url) {
    stop();
    const my = gen;
    canvas.hidden = false;
    const gl = glContext(canvas);
    if (!gl) throw new Error('webgl');
    gl.getExtension('OES_element_index_uint');
    const packed = await loadGltf(url);
    if (my !== gen) return;
    const gltf = packed.gltf;
    const bin = packed.bin;
    const views = gltf.bufferViews || [];
    const acc = gltf.accessors || [];
    const prim = gltf.meshes[0].primitives[0];
    const posA = acc[prim.attributes.POSITION];
    const pos = typed(bin, posA, views, 3);
    const nrm = prim.attributes.NORMAL !== undefined
      ? typed(bin, acc[prim.attributes.NORMAL], views, 3)
      : pos;
    const ia = acc[prim.indices];
    const idx = typed(bin, ia, views, 1);
    const node = (gltf.nodes && gltf.nodes[0]) || {};
    const nt = node.translation || [0, 0, 0];
    const min = posA.min || [-1, -1, -1];
    const max = posA.max || [1, 1, 1];
    const cx = (min[0] + max[0]) / 2 + nt[0];
    const cy = (min[1] + max[1]) / 2 + nt[1];
    const cz = (min[2] + max[2]) / 2 + nt[2];
    const span = Math.max(max[0] - min[0], max[1] - min[1], max[2] - min[2], 0.5);
    const color = (gltf.materials
      && gltf.materials[0]
      && gltf.materials[0].pbrMetallicRoughness
      && gltf.materials[0].pbrMetallicRoughness.baseColorFactor)
      || [0.62, 0.38, 0.22, 1];
    const clips = readClips(gltf, bin, acc, views);
    const aspect = (canvas.width || 1) / (canvas.height || 1);

    const vs = compile(gl, gl.VERTEX_SHADER, [
      'attribute vec3 aPos;',
      'attribute vec3 aNrm;',
      'uniform vec3 uCenter;',
      'uniform float uScale;',
      'uniform float uAngle;',
      'uniform float uAspect;',
      'uniform vec4 uQuat;',
      'uniform vec3 uTrans;',
      'varying vec3 vN;',
      'vec3 qrot(vec4 q, vec3 v) {',
      '  vec3 u = q.xyz;',
      '  return v + 2.0 * cross(u, cross(u, v) + q.w * v);',
      '}',
      'void main() {',
      '  vec3 p = qrot(uQuat, aPos) + uTrans;',
      '  p = (p - uCenter) * uScale;',
      '  float c = cos(uAngle);',
      '  float s = sin(uAngle);',
      '  vec3 r = vec3(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);',
      '  vec3 n = qrot(uQuat, aNrm);',
      '  vN = vec3(c * n.x + s * n.z, n.y, -s * n.x + c * n.z);',
      '  gl_Position = vec4(r.x * uAspect, r.y, r.z * 0.02, 1.0);',
      '}',
    ].join('\n'));
    const fs = compile(gl, gl.FRAGMENT_SHADER, [
      'precision mediump float;',
      'varying vec3 vN;',
      'uniform vec3 uCol;',
      'void main() {',
      '  vec3 n = normalize(vN);',
      '  float l = 0.35 + 0.65 * abs(dot(n, normalize(vec3(0.35, 0.8, 0.5))));',
      '  gl_FragColor = vec4(uCol * l, 1.0);',
      '}',
    ].join('\n'));
    const prog = gl.createProgram();
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.bindAttribLocation(prog, 0, 'aPos');
    gl.bindAttribLocation(prog, 1, 'aNrm');
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      throw new Error(gl.getProgramInfoLog(prog) || 'link');
    }

    const bufP = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, bufP);
    gl.bufferData(gl.ARRAY_BUFFER, pos, gl.STATIC_DRAW);
    const bufN = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, bufN);
    gl.bufferData(gl.ARRAY_BUFFER, nrm, gl.STATIC_DRAW);
    const bufI = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, bufI);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, idx, gl.STATIC_DRAW);

    gl.enable(gl.DEPTH_TEST);
    gl.clearColor(0.043, 0.051, 0.078, 1);
    const uCenter = gl.getUniformLocation(prog, 'uCenter');
    const uScale = gl.getUniformLocation(prog, 'uScale');
    const uAngle = gl.getUniformLocation(prog, 'uAngle');
    const uAspect = gl.getUniformLocation(prog, 'uAspect');
    const uQuat = gl.getUniformLocation(prog, 'uQuat');
    const uTrans = gl.getUniformLocation(prog, 'uTrans');
    const uCol = gl.getUniformLocation(prog, 'uCol');
    const scale = 1.7 / span;
    const indexType = ia.componentType === 5123 ? gl.UNSIGNED_SHORT : gl.UNSIGNED_INT;

    scene = {
      mode: 'still',
      t0: performance.now(),
      clips: clips,
      idleName: 'idle',
      actionName: 'headbutt-cycle',
    };

    function pose(now) {
      const elapsed = (now - scene.t0) / 1000;
      const idle = (scene.idleName && scene.clips[scene.idleName])
        || scene.clips.idle || scene.clips.sway;
      const action = (scene.actionName && scene.clips[scene.actionName])
        || scene.clips['headbutt-cycle'] || scene.clips.action || scene.clips.hit;
      if (scene.mode === 'action') {
        const dur = action && action.times && action.times.length
          ? action.times[action.times.length - 1]
          : 0.7;
        if (action && elapsed < dur) {
          const s = sampleClip(action, elapsed, false);
          return { q: s.q, p: s.p, yaw: 0.1 };
        }
        scene.mode = 'idle';
        scene.t0 = now;
      }
      if (scene.mode === 'idle') {
        if (idle) {
          const s = sampleClip(idle, elapsed, true);
          return { q: s.q, p: s.p, yaw: 0.2 };
        }
        return { q: [0, 0, 0, 1], p: [0, 0.04 * Math.sin(elapsed * 3.2), 0], yaw: elapsed * 0.6 };
      }
      return { q: [0, 0, 0, 1], p: [0, 0, 0], yaw: 0.35 };
    }

    function frame(now) {
      if (my !== gen || !scene) return;
      const po = pose(now);
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.useProgram(prog);
      gl.uniform3f(uCenter, cx, cy, cz);
      gl.uniform1f(uScale, scale);
      gl.uniform1f(uAngle, po.yaw);
      gl.uniform1f(uAspect, aspect);
      gl.uniform4f(uQuat, po.q[0], po.q[1], po.q[2], po.q[3]);
      gl.uniform3f(uTrans, po.p[0], po.p[1], po.p[2]);
      gl.uniform3f(uCol, color[0], color[1], color[2]);
      gl.bindBuffer(gl.ARRAY_BUFFER, bufP);
      gl.enableVertexAttribArray(0);
      gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, bufN);
      gl.enableVertexAttribArray(1);
      gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, bufI);
      gl.drawElements(gl.TRIANGLES, idx.length, indexType, 0);
      raf = requestAnimationFrame(frame);
    }
    raf = requestAnimationFrame(frame);
    frame(performance.now());
  }

  global.GltfView = { mount: mount, stop: stop, play: play };
}(typeof window !== 'undefined' ? window : this));
