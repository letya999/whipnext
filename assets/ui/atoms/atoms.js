window.atoms = {
  btn(label, extra) {
    const { id, kind, attrs } = extra || {};
    const cls = 'atom-btn' + (kind === 'primary' ? ' atom-btn-primary' : '');
    const ida = id ? ` id="${id}"` : '';
    return `<button type="button" class="${cls}"${ida} ${attrs || ''}>${label}</button>`;
  },
  sw(on, id) {
    const ida = id ? ` id="${id}"` : '';
    return `<button type="button" class="atom-sw${on ? ' on' : ''}"${ida} aria-pressed="${on}"><i></i></button>`;
  },
  input(opts) {
    const o = opts || {};
    return `<input class="atom-input" id="${o.id || ''}" value="${o.value || ''}" placeholder="${o.placeholder || ''}" />`;
  },
  select(opts) {
    const o = opts || {};
    const items = o.items || [];
    const cur = o.value;
    return `<select class="atom-select" id="${o.id || ''}" ${o.attrs || ''}>${
      items.map(v => `<option ${v === cur ? 'selected' : ''}>${v}</option>`).join('')
    }</select>`;
  },
  file(opts) {
    const o = opts || {};
    return `<input class="atom-file" type="file" id="${o.id || ''}" accept="${o.accept || '*'}"${o.multiple ? ' multiple' : ''}${o.attrs ? ` ${o.attrs}` : ''} />`;
  },
  range(opts) {
    const o = opts || {};
    return `<input class="atom-range" type="range" id="${o.id || ''}" min="${o.min || 25}" max="${o.max || 400}" value="${o.value || 100}" />`;
  },
  label(text) { return `<span class="atom-label">${text}</span>`; },
  hint(text) { return `<p class="atom-hint">${text}</p>`; },
  err(text) { return `<p class="atom-err">${text}</p>`; },
};
