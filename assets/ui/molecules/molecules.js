window.molecules = {
  field(label, control) {
    return `<label class="mol-field">${window.atoms.label(label)}${control}</label>`;
  },
  tile(m, on, isDefault, defaultLabel) {
    const kind = m.media_kind && m.media_kind !== 'image'
      ? `<i class="mol-badge">${m.media_kind}</i>`
      : '';
    const def = isDefault ? `<i class="mol-default">${defaultLabel}</i>` : '';
    const thumb = m.media_kind === '3d' || !m.preview
      ? `<span class="mol-3d-mark">3D</span>`
      : `<img src="${m.preview}" alt="" />`;
    return `<button type="button" class="mol-tile${on ? ' on' : ''}" data-model="${m.id}">
      ${thumb}
      ${kind}
      ${def}
      <span>${m.name}</span>
    </button>`;
  },
  choice(name, items, cur) {
    return `<div class="mol-choice">${items.map(([v, lab]) => `
      <label><input type="radio" name="${name}" value="${v}" ${v === cur ? 'checked' : ''}/> ${lab}</label>
    `).join('')}</div>`;
  },
  fileRole(label, id, accept, extra) {
    const o = extra || {};
    return `<div class="mol-file-role">${window.molecules.field(label, window.atoms.file({ id, accept, multiple: o.multiple }))}</div>`;
  },
  appRow(id, label, a, models, enabled, defaultLabel) {
    const cur = a.model || '';
    const opts = [`<option value="" ${cur === '' ? 'selected' : ''}>${defaultLabel}</option>`]
      .concat(models.map(m => `<option value="${m.id}" ${cur === m.id ? 'selected' : ''}>${m.name}</option>`));
    return `<div class="mol-app${enabled ? '' : ' off'}" data-id="${id}">
      <b>${label}</b>
      <select class="atom-select" data-k="model">${opts.join('')}</select>
      ${window.atoms.sw(!!a.enabled)}
    </div>`;
  },
};
