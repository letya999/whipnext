function esc(value) {
  return String(value == null ? '' : value).replace(/[&<>\"']/g, function (ch) {
    return ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '\"': '&quot;', "'": '&#39;' })[ch];
  });
}

function leaf(path) {
  return String(path || '').split(/[\\/]/).filter(Boolean).pop() || '';
}

function icon(name) {
  const paths = name === 'folder'
    ? '<path d="M3 6.5h6l2 2h10v10H3z"/><path d="M3 6.5v12"/>'
    : name === 'play'
      ? '<path d="m9 6 8 6-8 6z"/>'
      : '<path d="M4 17v3h3l10-10-3-3z"/><path d="m13 8 3 3"/>';
  return `<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${paths}</svg>`;
}

window.organisms = {
  folderInput(id, label, hint, accept, empty) {
    const locale = document.documentElement.lang === 'en' ? 'en' : 'ru';
    const choose = (window.I18N[locale] || window.I18N.ru).chooseFolder;
    return `<div class="folder-pick">
      <div class="folder-pick-head"><span class="atom-label">${label}</span><span class="folder-pick-hint">${hint}</span></div>
      <div class="folder-input-wrap"><button type="button" class="folder-button" id="${id}_button">${choose}</button><input class="folder-input-hidden" type="file" id="${id}" accept="${accept}" multiple webkitdirectory directory /></div>
      <p class="atom-hint" id="${id}_name">${empty}</p>
    </div>`;
  },
  location(label, path, openId, replaceId, replaceLabel, t, playId) {
    const buttons = `${playId ? `<button type="button" class="atom-btn atom-icon-btn" id="${playId}" title="${t('playSound')}" aria-label="${t('playSound')}">${icon('play')}</button>` : ''}<button type="button" class="atom-btn atom-icon-btn" id="${openId}" title="${t('openFolder')}" aria-label="${t('openFolder')}">${icon('folder')}</button><button type="button" class="atom-btn atom-icon-btn" id="${replaceId}" title="${replaceLabel}" aria-label="${replaceLabel}">${icon('replace')}</button>`;
    return `<div class="resource-field"><span class="resource-label">${label}</span><code class="resource-path" title="${esc(path)}">${esc(leaf(path) || path)}</code><div class="resource-actions">${buttons}</div></div>`;
  },
  mediaChoices(t, prefix) {
    const clipAccept = 'image/*,.gif,.svg,.glb,.gltf,.bin,video/*,.mp4,.webm,.mov,.avi';
    const stateAccept = 'image/*,.gif,.svg,video/*,.mp4,.webm,.mov,.avi';
    return `<details class="manual-assets">
      <summary>${t('pickFiles')}</summary>
      <div class="manual-assets-grid">
        ${window.molecules.fileRole(t('clip'), `${prefix}_media`, clipAccept, { multiple: true })}
        ${window.molecules.fileRole(t('stillImage'), `${prefix}_still`, 'image/*,video/*,.mp4')}
        ${window.molecules.fileRole(t('swayImage'), `${prefix}_idle`, stateAccept, { multiple: true })}
        ${window.molecules.fileRole(t('hitImage'), `${prefix}_action`, stateAccept, { multiple: true })}
      </div>
    </details>`;
  },
  editor(state, t, mode, m) {
    const add = mode === 'add';
    const prefix = add ? 'file' : 'file_replace';
    const name = add ? '' : esc((m && m.name) || '');
    const phrase = add ? '' : esc(((state.settings.phrases && m && state.settings.phrases[m.id]) || (m && m.phrase) || 'next'));
    const kind = add ? t('newCharacter') : ((m && m.media_kind) || 'image').toUpperCase();
    const isDefault = !add && m && state.settings.model === m.id;
    const defaultBtn = add ? '' : (isDefault
      ? `<span class="kind-chip">${t('alreadyDefault')}</span>`
      : window.atoms.btn(t('makeDefault'), { id: 'make_default' }));
    const folder = add ? t('folderEmpty') : ((m && m.folder_path) || (m && m.folder) || t('folderEmpty'));
    const sound = add ? '' : ((state.settings.model_sounds && m && state.settings.model_sounds[m.id]) || (m && m.sound_action) || '');
    const soundFile = (state.sound_files || []).find(x => x.stem === sound);
    const soundPath = soundFile ? soundFile.path : t('soundNone');
    const stage = add ? `<div class="editor-stage editor-stage-empty"><span>+</span><small>${t('previewAfterPick')}</small></div>` :
      `<div class="editor-stage">
        <img id="stage" alt="${name}" src="${m.preview || (m.media_kind === '3d' ? '' : ((m.idle && m.idle[0]) || ''))}" />
        <canvas id="stage3d" width="${state.layout.overlay_w}" height="${state.layout.overlay_h}" hidden></canvas>
      </div>`;
    const controls = add ? '' : `<div class="animation-stages"><span class="atom-label">${t('animationStages')}</span><div class="preview-controls">
      ${window.atoms.btn(t('stageStill'), { id: 'play_still' })}
      ${window.atoms.btn(t('stageSway'), { id: 'play_idle' })}
      ${window.atoms.btn(t('stageHit'), { id: 'play_action', kind: 'primary' })}
    </div></div>`;
    return `<div class="character-editor ${add ? 'is-new' : ''}">
      <div class="editor-heading"><h3>${add ? t('newCharacter') : name}</h3><span class="kind-chip">${kind}</span>${defaultBtn}</div>
      <div class="editor-grid">
        <div class="editor-preview">${stage}${controls}</div>
        <div class="editor-form">
          ${window.molecules.field(t('name'), window.atoms.input({ id: add ? 'new_name' : 'edit_name', value: name, placeholder: t('name') }))}
          ${window.molecules.field(t('phrase'), window.atoms.input({ id: add ? 'new_phrase' : 'edit_phrase', value: phrase, placeholder: 'next' }))}
          ${window.atoms.hint(t('phraseHint'))}
          ${add ? window.organisms.folderInput('file_folder', t('assetFolder'), t('folderHint'), 'image/*,.gif,.svg,.gltf,.glb,.bin,video/*,.mp4,.webm,.mov,.avi,.wav', t('folderEmpty')) : `<div class="resource-group">${window.organisms.location(t('animationFolder'), folder, 'open_character_folder', 'file_replace_folder_button', t('replaceFolder'), t)}${window.organisms.location(t('hitSound'), soundPath, 'open_sound_folder', 'replace_sound', t('replaceSound'), t, sound ? 'play_sound' : '')}</div><input class="folder-input-hidden" type="file" id="file_replace_folder" accept="image/*,.gif,.svg,.gltf,.glb,.bin,video/*,.mp4,.webm,.mov,.avi,.wav" multiple webkitdirectory directory /><input class="atom-file" type="file" id="file_sound" accept="audio/wav,.wav" hidden />`}
          ${add ? window.molecules.fileRole(t('hitSoundFile'), 'file_hit_sound', 'audio/wav,.wav') : ''}
          <div class="editor-actions">${window.atoms.btn(t('cancel'), { id: add ? 'cancel_add' : 'cancel_edit' })}${add ? window.atoms.btn(t('assemble'), { id: 'assemble', kind: 'primary' }) : window.atoms.btn(t('saveChanges'), { id: 'save_character', kind: 'primary' })}</div>
          <p class="atom-err" id="${add ? 'build_err' : 'replace_err'}" hidden></p>
        </div>
      </div>
      ${add ? `<details class="asset-panel"><summary><span>${t('files')}</span><span class="asset-count">0</span></summary><p class="empty-files">${t('folderFilesAfterSave')}</p></details>${window.organisms.mediaChoices(t, prefix)}` : ''}
      ${add ? window.atoms.hint(t('assembleHint')) : ''}
    </div>`;
  },
  builder(state, t, mode, selectedId) {
    const s = state.settings;
    const models = state.models || [];
    const sel = selectedId || s.model;
    const m = models.find(x => x.id === sel) || models[0];
    const tiles = models.map(x => window.molecules.tile(x, sel === x.id, s.model === x.id, t('default'))).join('')
      + `<button type="button" class="mol-tile mol-add" id="add_character" data-add="1"><span class="mol-add-plus">+</span><span>${t('add')}</span></button>`;
    return `<div class="org org-character" data-organism="character-builder">
      <div class="org-title-row"><h2>${t('characters')}</h2><span class="count-chip">${models.length}</span></div>
      <div class="org-grid" id="models">${tiles}</div>
      ${window.organisms.editor(state, t, mode, m)}
    </div>`;
  },
  harness(state, t) {
    const apps = Object.keys(state.settings.apps || {}).sort();
    const models = state.models || [];
    const rows = apps.length ? apps.map(id => {
      const a = state.settings.apps[id];
      return window.molecules.appRow(id, t(id) === id ? id : t(id), a, models, !!a.enabled, t('default'));
    }).join('') : `<p class="atom-hint">${t('noHarness')}</p>`;
    return `<div class="org" data-organism="harness-assign"><div class="org-title-row"><h2>${t('harness')}</h2></div><label class="toggle-row">${window.atoms.sw(!!state.settings.hide_now, 'hide_now')} ${t('hideNow')}</label><label class="toggle-row">${window.atoms.sw(!!state.settings.only_when_matched, 'only')} ${t('onlyMatched')}</label><div class="org-apps" id="apps">${rows}</div><div class="mol-row action-row">${window.atoms.btn(t('rescan'), { id: 'redetect', kind: 'primary' })}</div><h3 class="subheading">${t('herdr')}</h3><div class="org-apps" id="herdr">${(state.herdr && state.herdr.length) ? state.herdr.map(h => `<div class="mol-app${h.focused ? '' : ' off'}"><b>${h.name || h.pane_id}</b><span>${h.kind}${h.focused ? ' · ' + t('herdrFocused') : ''}</span><code>${h.pane_id}</code></div>`).join('') : `<p class="atom-hint">${t('noHerdr')}</p>`}</div></div>`;
  },
  overlay(state, t) {
    const s = state.settings; const L = state.layout; const mode = s.overlay_mode || 'always';
    return `<div class="org" data-organism="overlay-controls"><div class="org-title-row"><h2>${t('overlay')}</h2></div>${window.molecules.field(t('animSize') + ' (' + s.overlay_scale + '%)', window.atoms.range({ id: 'overlay_scale', min: L.scale_min, max: L.scale_max, value: s.overlay_scale }))}${window.molecules.field(t('controlSize') + ' (' + s.control_size + '%)', window.atoms.range({ id: 'control_size', min: L.control_scale_min, max: L.control_scale_max, value: s.control_size }))}${window.atoms.label(t('visibility'))}${window.molecules.choice('overlay_mode', [['always', t('visAlways')], ['while_held', t('visHeld')], ['hide_while_held', t('visHide')]], mode)}${window.molecules.field(t('holdKey'), window.atoms.select({ id: 'overlay_key', items: ['alt', 'ctrl', 'shift', 'win', 'space'], value: s.overlay_key || 'alt' }))}<label class="toggle-row">${window.atoms.sw(!!s.dev_mode, 'dev_mode')} ${t('devMode')}</label>${window.atoms.hint(t('devHint'))}</div>`;
  },
  foot(t) { return window.atoms.btn(t('close'), { id: 'quit', kind: 'primary' }); },
};
