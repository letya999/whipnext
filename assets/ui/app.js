(function () {
  let state = null;
  let timer = null;
  let frames = [];
  let fi = 0;
  let uiMode = 'inspect';
  let mounted3d = '';
  let previewWhich = 'still';
  let previewModel = '';
  let selectedId = '';

  function send(cmd, extra) {
    const payload = JSON.stringify(Object.assign({ cmd: cmd }, extra || {}));
    if (window.ipc && typeof window.ipc.postMessage === 'function') {
      window.ipc.postMessage(payload);
    }
  }
  function loc() {
    const v = state && state.settings && state.settings.locale;
    return v === 'en' ? 'en' : 'ru';
  }
  function t(key) {
    const pack = (window.I18N && window.I18N[loc()]) || {};
    const en = (window.I18N && window.I18N.en) || {};
    return pack[key] || en[key] || key;
  }
  function applyIncoming(s) {
    const prevDefault = state && state.settings && state.settings.model;
    state = s;
    const models = state.models || [];
    const def = state.settings && state.settings.model;
    const missing = selectedId && !models.some(function (m) { return m.id === selectedId; });
    if (!selectedId || missing || (def && def !== prevDefault)) {
      selectedId = def || (models[0] && models[0].id) || '';
    }
    render();
  }
  window.__whipnext_set = applyIncoming;

  function currentModel() {
    const id = selectedId || (state.settings && state.settings.model);
    return (state.models || []).find(m => m.id === id) || (state.models || [])[0];
  }
  function stopAnim() { if (timer) { clearInterval(timer); timer = null; } }
  function playFrames(list, once) {
    stopAnim();
    frames = list || [];
    fi = 0;
    const stage = document.getElementById('stage');
    if (!stage || !frames.length) return;
    stage.src = frames[0];
    const tick = state.layout.tick_ms;
    timer = setInterval(function () {
      fi += 1;
      if (fi >= frames.length) {
        if (once) { stopAnim(); fi = frames.length - 1; return; }
        fi = 0;
      }
      stage.src = frames[fi];
    }, tick);
  }
  function showPreview(m, which) {
    if (m.id !== previewModel) {
      previewModel = m.id;
      previewWhich = m.media_kind === '3d' ? 'still' : 'idle';
    }
    if (which) previewWhich = which;
    stopAnim();
    const img = document.getElementById('stage');
    const canvas = document.getElementById('stage3d');
    if (m.media_kind === '3d' && m.source && canvas && window.GltfView) {
      const L = state.layout;
      canvas.width = L.overlay_w;
      canvas.height = L.overlay_h;
      canvas.hidden = false;
      if (img) img.hidden = true;
      const go = function () {
        if (img) img.hidden = true;
        canvas.hidden = false;
        window.GltfView.play(previewWhich, {
          idle: (m.idle && m.idle[0]) || '',
          action: (m.action && m.action[0]) || '',
        });
      };
      if (mounted3d === m.source) { go(); return; }
      window.GltfView.mount(canvas, m.source).then(function () {
        mounted3d = m.source;
        go();
      }).catch(function () {
        mounted3d = '';
        canvas.hidden = true;
        if (img) img.hidden = false;
        playFrames(m.preview ? [m.preview] : [], true);
      });
      return;
    }
    mounted3d = '';
    if (window.GltfView) window.GltfView.stop();
    if (canvas) canvas.hidden = true;
    if (img) img.hidden = false;
    if (which === 'still') playFrames(m.preview ? [m.preview] : [], true);
    else if (which === 'action') playFrames(m.action, true);
    else playFrames(m.idle && m.idle.length ? m.idle : (m.preview ? [m.preview] : []), false);
  }
  function playStem(stem) {
    if (!stem) return;
    send('play', { sound: stem });
  }
  function push() { send('set', { settings: state.settings }); }

  function fileToMsgFile(f, role) {
    return new Promise(function (resolve) {
      if (!f) { resolve(null); return; }
      const r = new FileReader();
      r.onload = function () {
        const url = String(r.result || '');
        const i = url.indexOf('base64,');
        const b64 = i >= 0 ? url.slice(i + 7) : '';
        const lower = String(f.name || '').toLowerCase();
        const useRole = lower.endsWith('.bin') ? 'bin' : role;
        resolve({ role: useRole, name: f.webkitRelativePath || f.name, b64: b64 });
      };
      r.readAsDataURL(f);
    });
  }
  function fileToMsg(input, role) {
    const f = input && input.files && input.files[0];
    if (!f) return Promise.resolve(null);
    return fileToMsgFile(f, role);
  }
  function filesToMsgs(input, role) {
    const files = input && input.files ? Array.prototype.slice.call(input.files) : [];
    if (!files.length) return Promise.resolve([]);
    return Promise.all(files.map(function (f) { return fileToMsgFile(f, role); })).then(function (rows) {
      return rows.filter(Boolean);
    });
  }
  function folderRole(file) {
    const name = String(file.webkitRelativePath || file.name || '').toLowerCase();
    if (name.endsWith('.wav')) return 'sound';
    if (name.endsWith('.bin')) return 'bin';
    if (/(^|[\\/_-])(idle|sway|stand)([\\/_-]|\.)/.test(name)) return 'idle';
    if (/(^|[\\/_-])(action|hit|punch|swing)([\\/_-]|\.)/.test(name)) return 'action';
    return 'media';
  }
  function folderFilesToMsgs(input) {
    const files = input && input.files ? Array.prototype.slice.call(input.files) : [];
    if (!files.length) return Promise.resolve([]);
    return Promise.all(files.map(function (f) { return fileToMsgFile(f, folderRole(f)); }));
  }
  function setFolderLabel(id) {
    const input = document.getElementById(id);
    const label = document.getElementById(id + '_name');
    if (!input || !label || !input.files.length) return;
    const first = input.files[0].webkitRelativePath || input.files[0].name;
    const folder = first.split(/[\\/]/)[0];
    label.textContent = folder + ' · ' + input.files.length + ' ' + t('filesSelected');
  }

  function bind() {
    const $ = (id) => document.getElementById(id);
    document.documentElement.style.setProperty('--control-scale', String((state.settings.control_size || 100) / 100));
    document.documentElement.lang = loc();
    if ($('lang_ru')) $('lang_ru').onclick = function () {
      state.settings.locale = 'ru'; render(); push();
    };
    if ($('lang_en')) $('lang_en').onclick = function () {
      state.settings.locale = 'en'; render(); push();
    };
    if ($('only')) $('only').onclick = function () {
      state.settings.only_when_matched = !state.settings.only_when_matched; render(); push();
    };
    if ($('hide_now')) $('hide_now').onclick = function () {
      state.settings.hide_now = !state.settings.hide_now; render(); push();
    };
    if ($('dev_mode')) $('dev_mode').onclick = function () {
      state.settings.dev_mode = !state.settings.dev_mode; render(); push();
    };
    const models = $('models');
    if (models) models.onclick = function (e) {
      const add = e.target.closest('[data-add]');
      if (add) { uiMode = 'add'; render(); return; }
      const b = e.target.closest('[data-model]'); if (!b) return;
      uiMode = 'inspect';
      selectedId = b.dataset.model;
      render();
    };
    if ($('make_default')) $('make_default').onclick = function () {
      const m = currentModel();
      if (!m) return;
      state.settings.model = m.id;
      render(); push();
    };
    if ($('play_still')) $('play_still').onclick = function () {
      const m = currentModel(); if (m) showPreview(m, 'still');
    };
    if ($('play_idle')) $('play_idle').onclick = function () {
      const m = currentModel(); if (m) showPreview(m, 'idle');
    };
    if ($('play_action')) $('play_action').onclick = function () {
      const m = currentModel();
      if (!m) return;
      showPreview(m, 'action');
      playStem((state.settings.model_sounds && state.settings.model_sounds[m.id]) || m.sound_action);
    };
    if ($('play_sound')) $('play_sound').onclick = function () {
      const m = currentModel();
      if (!m) return;
      playStem((state.settings.model_sounds && state.settings.model_sounds[m.id]) || m.sound_action);
    };
    if ($('file_sound')) $('file_sound').onchange = async function () {
      const msg = await fileToMsg($('file_sound'), 'sound');
      if (!msg) return;
      const m = currentModel();
      if (m) send('replace-bytes', { id: m.id, files: [msg] });
    };
    ['file_folder', 'file_replace_folder'].forEach(function (id) {
      if ($(id)) {
        $(id).onchange = function () { setFolderLabel(id); };
        if ($(id + '_button')) $(id + '_button').onclick = function () { $(id).click(); };
      }
    });
    if ($('overlay_scale')) $('overlay_scale').oninput = function (e) {
      const L = state.layout;
      let n = Number(e.target.value);
      if (n < L.scale_min) n = L.scale_min;
      if (n > L.scale_max) n = L.scale_max;
      state.settings.overlay_scale = n; push();
    };
    if ($('control_size')) $('control_size').oninput = function (e) {
      const L = state.layout;
      let n = Number(e.target.value);
      if (n < L.control_scale_min) n = L.control_scale_min;
      if (n > L.control_scale_max) n = L.control_scale_max;
      state.settings.control_size = n;
      document.documentElement.style.setProperty('--control-scale', String(state.settings.control_size / 100));
      push();
    };
    document.querySelectorAll('input[name="overlay_mode"]').forEach(function (el) {
      el.onchange = function () { state.settings.overlay_mode = el.value; push(); };
    });
    if ($('overlay_key')) $('overlay_key').onchange = function (e) {
      state.settings.overlay_key = e.target.value; push();
    };
    const apps = $('apps');
    if (apps) {
      apps.onclick = function (e) {
        const row = e.target.closest('.mol-app'); if (!row) return;
        if (e.target.closest('.atom-sw')) {
          const id = row.dataset.id;
          state.settings.apps[id].enabled = !state.settings.apps[id].enabled;
          render(); push();
        }
      };
      apps.onchange = function (e) {
        const row = e.target.closest('.mol-app'); if (!row || e.target.tagName !== 'SELECT') return;
        const id = row.dataset.id; let v = e.target.value;
        state.settings.apps[id].model = v ? v : null;
        push();
      };
    }
    if ($('save_character')) $('save_character').onclick = async function () {
      const m = currentModel();
      if (!m) return;
      const name = (($('edit_name') && $('edit_name').value) || '').trim();
      const err = $('replace_err');
      if (!name) {
        if (err) { err.hidden = false; err.textContent = t('needName'); }
        return;
      }
      const phrase = (($('edit_phrase') && $('edit_phrase').value) || '').trim();
      const folder = await folderFilesToMsgs($('file_replace_folder'));
      const files = folder.length ? folder : (await Promise.all([
        filesToMsgs($('file_replace_media'), 'media'),
        filesToMsgs($('file_replace_idle'), 'idle'),
        filesToMsgs($('file_replace_action'), 'action'),
      ])).flat().filter(Boolean);
      if (name !== m.name) {
        send('rename', { id: m.id, name: name });
        m.name = name;
        const heading = document.querySelector('.editor-heading h3');
        const tileName = document.querySelector(`[data-model="${m.id}"] > span:last-child`);
        if (heading) heading.textContent = name;
        if (tileName) tileName.textContent = name;
      }
      if (!state.settings.phrases) state.settings.phrases = {};
      state.settings.phrases[m.id] = phrase || 'next';
      send('replace-bytes', { id: m.id, phrase: phrase || 'next', files: files });
      push();
      if (err) err.hidden = true;
    };
    if ($('cancel_add') || $('cancel_edit')) {
      const cancel = $('cancel_add') || $('cancel_edit');
      cancel.onclick = function () { uiMode = 'inspect'; render(); };
    }
    if ($('assemble')) $('assemble').onclick = async function () {
      const name = ($('new_name') && $('new_name').value) || '';
      const err = $('build_err');
      if (!name.trim()) {
        if (err) { err.hidden = false; err.textContent = t('needName'); }
        return;
      }
      const folder = await folderFilesToMsgs($('file_folder'));
      const files = folder.length ? folder : (await Promise.all([
        filesToMsgs($('file_media'), 'media'),
        fileToMsg($('file_still'), 'still'),
        filesToMsgs($('file_idle'), 'idle'),
        filesToMsgs($('file_action'), 'action'),
      ])).flat().filter(Boolean);
      const sound = await fileToMsg($('file_hit_sound'), 'sound');
      if (sound) files.push(sound);
      if (!files.length) {
        if (err) { err.hidden = false; err.textContent = t('needFiles'); }
        return;
      }
      if (err) err.hidden = true;
      uiMode = 'inspect';
      const phrase = (($('new_phrase') && $('new_phrase').value) || '').trim();
      send('import-bytes', { name: name, phrase: phrase || 'next', files: files });
    };
    if ($('redetect')) $('redetect').onclick = function () { send('redetect'); };
    if ($('open_character_folder')) $('open_character_folder').onclick = function () {
      const m = currentModel(); if (m) send('open-character-folder', { id: m.id });
    };
    if ($('replace_sound')) $('replace_sound').onclick = function () {
      const input = $('file_sound'); if (input) input.click();
    };
    if ($('open_sound_folder')) $('open_sound_folder').onclick = function () { send('open-sound-folder'); };
    if ($('quit')) $('quit').onclick = function () { send('quit'); };
  }

  function render() {
    if (!state) return;
    if (state.logo) {
      const logo = document.getElementById('logo');
      if (logo) logo.src = state.logo;
    }
    const title = document.getElementById('ui_title');
    const tag = document.getElementById('ui_tagline');
    if (title) title.textContent = t('title');
    if (tag) tag.innerHTML = t('tagline').replace('next', '<code>next</code>');
    const ru = document.getElementById('lang_ru');
    const en = document.getElementById('lang_en');
    if (ru) ru.className = 'atom-btn' + (loc() === 'ru' ? ' atom-btn-primary' : '');
    if (en) en.className = 'atom-btn' + (loc() === 'en' ? ' atom-btn-primary' : '');
    const parked = document.getElementById('stage3d');
    if (parked) {
      parked.id = 'stage3d-keep';
      parked.hidden = true;
      document.body.appendChild(parked);
    }
    document.getElementById('org-builder').innerHTML = window.organisms.builder(state, t, uiMode, selectedId);
    document.getElementById('org-harness').innerHTML = window.organisms.harness(state, t);
    document.getElementById('org-overlay').innerHTML = window.organisms.overlay(state, t);
    document.getElementById('org-foot').innerHTML = window.organisms.foot(t);
    const m = currentModel();
    const slot = document.getElementById('stage3d');
    const reuse = parked && slot && m && m.media_kind === '3d' && m.source === mounted3d && uiMode === 'inspect';
    if (reuse) {
      slot.parentNode.replaceChild(parked, slot);
      parked.id = 'stage3d';
      parked.hidden = false;
      const img = document.getElementById('stage');
      if (img) img.hidden = true;
    } else if (parked) {
      parked.remove();
      mounted3d = '';
      if (window.GltfView) window.GltfView.stop();
    }
    bind();
    if (m && uiMode === 'inspect' && !reuse) {
      showPreview(m, m.media_kind === '3d' ? previewWhich : 'idle');
    }
  }

  if (window.__BOOT) applyIncoming(window.__BOOT);
  (function boot() {
    if (window.ipc) send('ready');
    else setTimeout(boot, 50);
  })();
})();
