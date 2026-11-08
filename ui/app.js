'use strict';
// ValSync frontend. Runs inside Tauri (commands via invoke) or in a plain browser with mock data.
const tauri = window.__TAURI__;
const invoke = (cmd, args) => tauri.core.invoke(cmd, args);
const $ = s => document.querySelector(s);
const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

/* ---------- Icons (20px grid, 1.6 stroke) ---------- */
const IC = {
  transfer: '<path d="M3.5 7h12l-3.2-3.2M16.5 13h-12l3.2 3.2"/>',
  diff: '<rect x="3" y="3.5" width="14" height="13" rx="2.5"/><path d="M10 3.5v13M5.8 8h2M12.2 8h2M5.8 11.5h2M12.2 11.5h2"/>',
  crosshair: '<circle cx="10" cy="10" r="6.5"/><path d="M10 1.8V6M10 14v4.2M1.8 10H6M14 10h4.2"/>',
  backups: '<path d="M3.5 10a6.5 6.5 0 1 0 1.9-4.6L3.5 7.3"/><path d="M3.5 3.8v3.5H7M10 6.5V10l2.5 1.6"/>',
  sensitivity: '<rect x="5.5" y="2.5" width="9" height="15" rx="4.5"/><path d="M10 5.5v3"/>',
  keybinds: '<rect x="2.5" y="5" width="15" height="10" rx="2"/><path d="M5.5 8h.01M8.5 8h.01M11.5 8h.01M14.5 8h.01M6.5 12h7"/>',
  video: '<rect x="2.5" y="3.5" width="15" height="10" rx="1.8"/><path d="M7 17h6M10 13.5V17"/>',
  audio: '<path d="M3.5 8v4h3l4 3.5v-11l-4 3.5zM13.5 7.5a3.5 3.5 0 0 1 0 5M15.5 5a7 7 0 0 1 0 10"/>',
  minimap: '<path d="M2.5 5.5l5-2 5 2 5-2v11l-5 2-5-2-5 2zM7.5 3.5v11M12.5 5.5v11"/>',
  hud: '<rect x="2.5" y="3.5" width="15" height="13" rx="2"/><path d="M5.5 13.5h3M12 13.5h2.5M5.5 6.5h2"/>',
  check: '<path d="M5 10.5l3.2 3L15 6.5"/>',
  chev: '<path d="M8 5l5 5-5 5"/>',
  swap: '<path d="M6 3.5l-3 3 3 3M3 6.5h11M14 10.5l3 3-3 3M17 13.5H6"/>',
  copy: '<rect x="7" y="7" width="10" height="10" rx="2"/><path d="M13 7V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2"/>',
  paste: '<rect x="4.5" y="3.5" width="11" height="14" rx="2"/><path d="M7.5 3.5v-1h5v1M7.5 9h5M7.5 12h3"/>',
  shield: '<path d="M10 2.5l6 2.3v4.7c0 3.8-2.6 6.6-6 8-3.4-1.4-6-4.2-6-8V4.8z"/><path d="M7.3 10l2 2 3.5-3.7"/>',
  snap: '<path d="M5.5 2.5h9v15l-4.5-3.2-4.5 3.2z"/>',
  imported: '<path d="M3.5 12.5v3h13v-3M10 3.5v9M6.5 9l3.5 3.5L13.5 9"/>',
  folder: '<path d="M2.5 6A1.5 1.5 0 0 1 4 4.5h3.5l1.8 2H16A1.5 1.5 0 0 1 17.5 8v6.5A1.5 1.5 0 0 1 16 16H4a1.5 1.5 0 0 1-1.5-1.5z"/>',
  plus: '<path d="M10 4v12M4 10h12"/>',
  info: '<circle cx="10" cy="10" r="7"/><path d="M10 9v4.5M10 6.5h.01"/>',
  arrow: '<path d="M4 10h12M11.5 5.5L16 10l-4.5 4.5"/>',
};
const ic = (n, s = 18, w = 1.6) => `<svg width="${s}" height="${s}" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round">${IC[n]}</svg>`;
IC.applied = IC.transfer; IC['auto-backup'] = IC.shield; IC.manual = IC.snap;

/* ---------- Mock data (mirrors src-tauri/src/sample.rs) ---------- */
const NOW = new Date(2026, 10, 8, 14, 40).getTime() / 1000;
const at = (d, h, m) => new Date(2026, 10, d, h, m).getTime() / 1000;
const MOCK_ACCOUNTS = [
  ['nightjar-euw1', 'Nightjar#EUW1', 'EU', 'Immortal 1', 287, NOW - 130],
  ['kestrel-2290', 'Kestrel#2290', 'EU', 'Platinum 1', 131, NOW - 6 * 86400],
  ['nightjar-smurf-7781', 'nightjar smurf#7781', 'EU', 'Gold 3', 64, NOW - 3 * 86400],
  ['halcyon-tr01', 'Halcyon#TR01', 'EU', 'Diamond 2', 176, NOW - 15 * 86400],
  ['quietpeak-na5', 'quietpeak#NA5', 'NA', 'Silver 2', 41, null],
].map(([id, riotId, region, rank, level, lastSynced], i) => ({ id, riotId, region, rank, level, lastSynced, isMain: i === 0, rankInitial: rank[0] }));
const CAT_META = {
  crosshair: ['Crosshair', 'Primary, Dot and Op profile codes'],
  sensitivity: ['Sensitivity', '0.32 at 800 DPI · eDPI 256 · scoped 1.00'],
  keybinds: ['Keybinds', 'Movement, abilities, comms, buy menu'],
  video: ['Video', '1920×1080 · 240 Hz · Low · MSAA 4x'],
  audio: ['Audio', 'Master 62% · music off · HRTF'],
  minimap: ['Minimap', 'Rotate · size 1.10 · zoom 0.90'],
  hud: ['HUD', 'Corpses and blood off · tracers on'],
};
const MOCK_CATS = [['crosshair', 3, 'profiles'], ['sensitivity', 6, 'settings'], ['keybinds', 64, 'bindings'], ['video', 18, 'settings'], ['audio', 14, 'settings'], ['minimap', 7, 'settings'], ['hud', 12, 'settings']]
  .map(([category, count, unit]) => ({ category, label: CAT_META[category][0], count, unit }));
const V = (t, value) => value === undefined ? null : ({ b: { type: 'bool', value }, f: { type: 'float', value }, i: { type: 'int', value }, t: { type: 'text', value } })[t];
const MOCK_DIFF = [
  ['crosshair', 'Primary - cyan small', 't', '0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0', '0;P;c;1;0t;1;0l;3;0o;2;1b;0'],
  ['crosshair', 'Dot - violet', 't', '0;P;c;8;u;8E7CFFFF;h;0;d;1;z;3;0b;0;1b;0', '0;P;c;8;u;8E7CFFFF;h;0;d;1;z;3;0b;0;1b;0'],
  ['crosshair', 'Op - red plus', 't', '0;P;c;7;t;2;o;1;0t;3;0l;5;0o;1;0a;1;1b;0;S;c;0', '0;P;c;7;t;2;o;1;0t;3;0l;5;0o;1;0a;1;1b;0;S;c;0'],
  ['sensitivity', 'MouseSensitivity', 'f', 0.32, 0.4], ['sensitivity', 'ScopedSensitivityMultiplier', 'f', 1, 0.95],
  ['sensitivity', 'AdsSensitivityMultiplier', 'f', 1, 1], ['sensitivity', 'RawInputBuffer', 'b', true, false], ['sensitivity', 'MouseDpi', 'i', 800, 800],
  ['keybinds', 'Ability1', 't', 'C', 'Q'], ['keybinds', 'Ability2', 't', 'Q', 'C'], ['keybinds', 'Ability3', 't', 'E', 'E'], ['keybinds', 'Ultimate', 't', 'X', 'X'],
  ['keybinds', 'Ping', 't', 'Middle Mouse Button', 'Z'], ['keybinds', 'Walk', 't', 'Left Shift', 'Left Shift'], ['keybinds', 'PushToTalkTeam', 't', 'V', 'V'],
  ['keybinds', 'ChatParty', 't', 'P', undefined],
  ['video', 'AntiAliasing', 't', 'MSAA 4x', 'FXAA'], ['video', 'NvidiaReflex', 't', 'On + Boost', 'On'], ['video', 'Bloom', 't', 'Off', 'On'],
  ['video', 'Resolution', 't', '1920x1080 16:9 (240Hz)', '1920x1080 16:9 (240Hz)'], ['video', 'MaterialQuality', 't', 'Low', 'Low'],
  ['audio', 'OverallVolume', 'f', 0.62, 0.8], ['audio', 'Music', 'f', 0, 0.35], ['audio', 'SoundEffects', 'f', 1, 1], ['audio', 'VoiceOver', 'f', 0.85, 0.85],
  ['minimap', 'Rotate', 'b', true, false], ['minimap', 'FixedOrientation', 't', 'Based on Side', 'Always the Same'], ['minimap', 'KeepPlayerCentered', 'b', false, true],
  ['minimap', 'Size', 'f', 1.1, 1], ['minimap', 'Zoom', 'f', 0.9, 0.85], ['minimap', 'VisionCones', 'b', true, true],
  ['hud', 'ShowCorpses', 'b', false, true], ['hud', 'ShowBlood', 'b', false, true], ['hud', 'ShowBulletTracers', 'b', true, true], ['hud', 'SprayInSpectator', 't', undefined, 'On'],
].map(([category, key, t, s, g]) => {
  const source = V(t, s), target = V(t, g);
  const kind = !target ? 'added' : !source ? 'removed' : JSON.stringify(source) === JSON.stringify(target) ? 'same' : 'changed';
  return { category, key, source, target, kind };
});
const cnt = o => Object.fromEntries(MOCK_CATS.map(c => [c.category, o ? o[c.category] ?? c.count : c.count]));
const MOCK_BACKUPS = [
  ['b1', 'kestrel-2290', at(8, 14, 31), 'Auto-backup before transfer from Nightjar#EUW1', 'auto-backup', 124, 18840],
  ['b2', 'nightjar-smurf-7781', at(8, 14, 31), 'Auto-backup before transfer from Nightjar#EUW1', 'auto-backup', 124, 18712],
  ['b3', 'halcyon-tr01', at(8, 14, 31), 'Auto-backup before transfer from Nightjar#EUW1', 'auto-backup', 123, 18690],
  ['b4', 'nightjar-euw1', at(8, 12, 5), 'Before patch 11.10', 'manual', 124, 18902],
  ['b5', 'kestrel-2290', at(7, 21, 48), 'Cherry-picked 6 settings from Nightjar#EUW1', 'applied', 124, 18833],
  ['b6', 'kestrel-2290', at(7, 21, 48), 'Auto-backup before cherry-pick from Nightjar#EUW1', 'auto-backup', 124, 18801],
  ['b7', 'quietpeak-na5', at(7, 18, 12), 'Imported from this PC', 'imported', 118, 17420],
  ['b8', 'nightjar-euw1', at(5, 16, 40), 'Tournament setup, 1.1 minimap', 'manual', 124, 18899],
  ['b9', 'nightjar-smurf-7781', at(5, 9, 2), 'Auto-backup before restoring "Ranked default"', 'auto-backup', 121, 18120],
].map(([id, accountId, createdAt, label, kind, total, bytes]) => ({ id, accountId, createdAt, label, kind, total, bytes, counts: cnt(total < 124 ? { keybinds: 64 - (124 - total) } : null) }));
const MOCK_XH = [
  ['Primary - cyan small', '0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0'],
  ['Dot - violet', '0;P;c;8;u;8E7CFFFF;h;0;d;1;z;3;0b;0;1b;0'],
  ['Op - red plus', '0;P;c;7;t;2;o;1;0t;3;0l;5;0o;1;0a;1;1b;0;S;c;0'],
].map(([name, code]) => ({ name, code }));
const RECENT = [['0;P;c;1;o;1;d;1;z;3;0b;0;1b;0', '2 h ago'], ['0;s;1;P;c;8;u;FF4655FF;h;0;0l;3;0o;2;0a;1;0f;0;1b;0', 'Yesterday'], ['0;P;h;0;f;0;0l;4;0o;0;0a;1;0f;0;1b;0', '3 days ago']];

/* ---------- State ---------- */
const S = {
  screen: 'transfer', accounts: MOCK_ACCOUNTS, cats: MOCK_CATS, diff: MOCK_DIFF, backups: MOCK_BACKUPS, xhs: MOCK_XH,
  vaultPath: '%APPDATA%\\ValSync', riot: { clientRunning: true, gameRunning: false, apiPort: 51537 }, now: NOW,
  source: 'nightjar-euw1', targets: ['kestrel-2290'], sel: new Set(['crosshair', 'sensitivity', 'keybinds', 'video', 'minimap', 'hud']),
  autoBackup: true, skipSame: true, picker: null, pickHover: null, tx: { phase: 'idle', p: 0 },
  dcat: 'all', onlyChanged: false, dsel: null, dhover: null,
  xsel: 0, code: null, focus: false, zoom: 3, bk: 'b1', bkFilter: 'all',
};
const acc = id => S.accounts.find(a => a.id === id) || S.accounts[0];
const keyOf = e => e.category + '/' + e.key;
const pending = () => S.diff.filter(e => e.kind !== 'same');
if (!S.dsel) S.dsel = new Set(pending().map(keyOf));

/* ---------- Formatting ---------- */
const trim = x => String(+(+x).toFixed(3));
const pretty = k => k.replace(/([a-z])([A-Z0-9])/g, '$1 $2');
function ago(t) {
  if (!t) return 'Never';
  const d = S.now - t;
  if (d < 60) return 'Just now';
  if (d < 3600) return Math.round(d / 60) + ' min ago';
  if (d < 86400) return Math.round(d / 3600) + ' h ago';
  const n = Math.round(d / 86400);
  return n === 1 ? 'Yesterday' : n + ' days ago';
}
function fmtVal(cat, v) {
  if (!v) return '<span class="faint">Not set</span>';
  const x = v.value;
  if (v.type === 'bool') return x ? 'On' : 'Off';
  if (v.type === 'float') return cat === 'audio' ? Math.round(x * 100) + '%' : (+x).toFixed(2);
  if (cat === 'crosshair') return `<span class="mono">${esc(x)}</span>`;
  return esc(x);
}
const rid = r => { const [n, t] = r.split('#'); return `${esc(n)}<span>#${esc(t)}</span>`; };
const av = (a, cls = '') => `<span class="av ${a.isMain ? 'mn' : ''} ${cls}">${esc(a.rankInitial)}</span>`;
const kb = b => b > 1e6 ? (b / 1e6).toFixed(1) + ' MB' : (b / 1024).toFixed(1) + ' KB';
const hm = t => { const d = new Date(t * 1000); return String(d.getHours()).padStart(2, '0') + ':' + String(d.getMinutes()).padStart(2, '0'); };

/* ---------- Crosshair codes (mirror of crosshair.rs for instant preview) ---------- */
const PALETTE = ['#FFFFFF', '#00FF00', '#7FFF00', '#DFFF00', '#FFFF00', '#00FFFF', '#FF00FF', '#FF0000'];
const PNAMES = ['White', 'Green', 'Yellow Green', 'Green Yellow', 'Yellow', 'Cyan', 'Pink', 'Red'];
const XDEF = () => ({ color: 0, custom: null, outlines: true, ot: 1, oo: 0.5, dot: false, dz: 2, da: 1,
  inner: { show: true, t: 2, l: 6, o: 3, a: 0.8, m: false, f: true }, outer: { show: true, t: 2, l: 2, o: 10, a: 0.35, m: true, f: true } });
function parseXh(code) {
  const tk = code.trim().replace(/;+$/, '').split(';');
  if (!/^\d+$/.test(tk[0] || '')) throw 'Code must start with a version number';
  const p = XDEF(); let sec = 'G'; p.n = 0;
  for (let i = 1; i < tk.length; i++) {
    const k = tk[i];
    if (k === 'P' || k === 'A' || k === 'S') { sec = k; continue; }
    const v = tk[++i];
    if (v === undefined || v === '') throw `Key "${k}" has no value yet`;
    if (sec !== 'P') continue;
    const f = +v; p.n++;
    if (k !== 'u' && Number.isNaN(f)) throw `"${v}" is not a number`;
    if (k === 'c') { if (f > 8) throw 'Color index out of range'; p.color = f; }
    else if (k === 'u') p.custom = '#' + v.slice(0, 6);
    else if ('htodza'.includes(k) && k.length === 1) p[{ h: 'outlines', t: 'ot', o: 'oo', d: 'dot', z: 'dz', a: 'da' }[k]] = 'hd'.includes(k) ? v === '1' : f;
    else if (/^[01][btloamf]$/.test(k)) { const L = k[0] === '0' ? p.inner : p.outer, c = k[1]; L[c === 'b' ? 'show' : c] = 'bmf'.includes(c) ? v === '1' : f; }
  }
  return p;
}
const xhHex = p => p.color === 8 && p.custom ? p.custom : PALETTE[p.color] || '#FFFFFF';
function xhSvg(p, s, W) {
  const c = W / 2, col = xhHex(p), out = [], fill = [];
  const r = (x, y, w, h, a) => {
    if (p.outlines) { const o = p.ot * s; out.push(`<rect x="${x - o}" y="${y - o}" width="${w + 2 * o}" height="${h + 2 * o}" fill="#000" opacity="${p.oo * Math.min(1, a * 1.6)}"/>`); }
    fill.push(`<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${col}" opacity="${a}"/>`);
  };
  for (const L of [p.inner, p.outer]) {
    if (!L.show || !L.l || !L.t) continue;
    const t = L.t * s, l = L.l * s, o = L.o * s;
    r(c + o, c - t / 2, l, t, L.a); r(c - o - l, c - t / 2, l, t, L.a); r(c - t / 2, c - o - l, t, l, L.a); r(c - t / 2, c + o, t, l, L.a);
  }
  if (p.dot) { const z = p.dz * s; r(c - z / 2, c - z / 2, z, z, p.da); }
  return `<svg width="${W}" height="${W}" viewBox="0 0 ${W} ${W}" shape-rendering="crispEdges">${out.join('')}${fill.join('')}</svg>`;
}

/* ---------- Sidebar ---------- */
const NAV = [['transfer', 'Transfer', 'transfer'], ['diff', 'Review changes', 'diff'], ['crosshairs', 'Crosshairs', 'crosshair'], ['backups', 'Backups', 'backups']];
function sidebar() {
  const badge = { diff: `<span class="badge">${S.dsel.size}</span>`, crosshairs: `<span class="badge gray">${S.xhs.length}</span>`, backups: `<span class="badge gray">${S.backups.length}</span>` };
  const total = S.backups.reduce((n, b) => n + b.bytes, 0);
  return NAV.map(([id, label, icon]) => `<a class="nav ${S.screen === id ? 'on' : ''}" href="#${id}">${ic(icon)}${label}${badge[id] || ''}</a>`).join('') +
    `<div class="sb-h">Accounts <button class="link" data-act="add">${ic('plus', 14, 1.8)}</button></div>` +
    S.accounts.map(a => `<div class="sb-acc">${av(a)}<div style="min-width:0"><div class="n">${esc(a.riotId)}</div><div class="s">${a.region} · ${esc(a.rank)}${a.isMain ? ' · Main' : ''}</div></div></div>`).join('') +
    `<div class="sb-foot"><div style="display:flex;gap:8px;align-items:center">${ic('folder', 15)}<b class="mono" style="font-size:11.5px">${esc(S.vaultPath)}</b></div>
     <div class="num">${S.backups.length} snapshots · ${kb(total)} on disk</div>
     <div style="display:flex;gap:8px;align-items:center">${ic('shield', 15)}No Riot credentials stored</div></div>`;
}

/* ---------- Transfer ---------- */
function accCard(a, role, extra) {
  const done = S.tx.phase === 'done' && role === 'to';
  const stack = role === 'to' && S.targets.length > 1;
  return `<div class="acc-card ${stack ? 'stack' : ''} ${done ? 'done' : ''}" data-card="${role}">
    <div class="lab">${role === 'from' ? 'From' : 'To'}${stack ? ` · ${S.targets.length} accounts` : ''}<button class="link" data-act="pick" data-id="${role}">Change</button></div>
    <div class="who">${av(a, 'lg')}<div style="min-width:0"><div class="rid">${rid(a.riotId)}</div><div class="meta">${a.region} · ${esc(a.rank)} · Level ${a.level}</div></div></div>
    <div class="foot"><span>${role === 'from' ? 'Main account' : extra || 'Last synced'}</span><b>${done ? 'Synced just now' : ago(a.lastSynced)}</b></div></div>`;
}
function connector() {
  const { phase, p } = S.tx, W = 210, y = 60, n = S.cats.filter(c => S.sel.has(c.category)).reduce((s, c) => s + c.count, 0);
  const prog = phase === 'done' ? 1 : phase === 'run' ? p : 0;
  const pulses = phase === 'run' ? [0, 1, 2].map(i => { const x = ((p * 5 + i / 3) % 1) * W * prog; return `<circle cx="${x}" cy="${y}" r="3" fill="#A193FF" opacity="${0.4 + 0.6 * (x / W)}"/>`; }).join('') : '';
  const cur = S.cats.filter(c => S.sel.has(c.category))[Math.min(S.sel.size - 1, Math.floor(p * S.sel.size))];
  const cap = phase === 'run' ? `Copying ${cur ? cur.label : ''} · <b class="num">${Math.round(p * 100)}%</b>`
    : phase === 'done' ? `<b>Done</b> · ${S.targets.length} account${S.targets.length > 1 ? 's' : ''} updated` : `${S.sel.size} categories · ${S.autoBackup ? 'backup first' : 'no backup'}`;
  return `<div class="conn"><div class="cap-top num">${n} settings</div>
    <svg width="${W}" height="120"><line x1="8" y1="${y}" x2="${W - 8}" y2="${y}" stroke="rgba(255,255,255,.1)" stroke-width="2" stroke-dasharray="2 6" stroke-linecap="round"/>
    ${phase === 'idle' ? `<line class="flow" x1="8" y1="${y}" x2="${W - 8}" y2="${y}" stroke="#8E7CFF" stroke-opacity=".55" stroke-width="2" stroke-dasharray="2 10" stroke-linecap="round"/>` : `<line x1="8" y1="${y}" x2="${8 + (W - 16) * prog}" y2="${y}" stroke="#8E7CFF" stroke-width="2" stroke-linecap="round"/>`}
    ${pulses}<circle cx="8" cy="${y}" r="3.5" fill="#8E7CFF"/><circle cx="${W - 8}" cy="${y}" r="3.5" fill="${prog >= 1 ? '#8E7CFF' : '#2A2B31'}"/></svg>
    <div class="node ${phase === 'done' ? 'done' : ''}">${ic(phase === 'done' ? 'check' : 'arrow', 20, 1.8)}</div><div class="cap-t">${cap}</div></div>`;
}
function picker() {
  if (!S.picker) return '';
  const list = S.accounts.filter(a => S.picker === 'from' ? true : a.id !== S.source);
  return `<div class="pop" id="pop" data-for="${S.picker}"><div class="pop-h">${S.picker === 'from' ? 'Copy settings from' : 'Copy settings to'}</div>` +
    list.map(a => { const on = S.picker === 'from' ? a.id === S.source : S.targets.includes(a.id); return `<div class="row click ${S.pickHover === a.id ? 'hl' : ''}" data-act="choose" data-id="${a.id}">${av(a)}<div><div class="t">${esc(a.riotId)}</div><div class="faint" style="font-size:11.5px">${a.region} · ${esc(a.rank)}</div></div><span class="end">${on ? `<span style="color:var(--accent)">${ic('check', 16, 2)}</span>` : ''}</span></div>`; }).join('') + '</div>';
}
function transfer() {
  const src = acc(S.source), tgt = acc(S.targets[0]), alts = S.accounts.filter(a => a.id !== S.source);
  const nSel = S.cats.filter(c => S.sel.has(c.category)).reduce((s, c) => s + c.count, 0), all = S.cats.reduce((s, c) => s + c.count, 0);
  const btn = S.tx.phase === 'run' ? `Transferring… <span class="num">${Math.round(S.tx.p * 100)}%</span>` : S.tx.phase === 'done' ? `${ic('check', 16, 2)} Done`
    : `Transfer to ${S.targets.length > 1 ? S.targets.length + ' accounts' : esc(tgt.riotId)}`;
  return `<section class="screen" id="s-transfer">
  <div class="head"><div><h1>Transfer settings</h1><p>Copy your setup from one Riot account to another. Targets are backed up before anything is overwritten.</p></div><div class="sp"></div>
  <a class="btn" href="#diff">${ic('diff', 16)}Review ${S.dsel.size} changes</a></div>
  <div class="hero">${accCard(src, 'from')}${connector()}${accCard(tgt, 'to', S.targets.length > 1 ? '+ ' + S.targets.slice(1).map(id => acc(id).riotId.split('#')[0]).join(', ') : '')}</div>
  <div class="cols"><div>
    <div class="group-t"><span>What to transfer</span><button class="link" data-act="allcats">${S.sel.size === S.cats.length ? 'Deselect all' : 'Select all'}</button></div>
    <div class="group">${S.cats.map(c => { const on = S.sel.has(c.category); return `<div class="row click ${on ? 'sel' : ''}" data-act="cat" data-id="${c.category}" style="min-height:50px">
      <span class="tile">${ic(c.category, 16)}</span><div style="min-width:0"><div><span class="t">${c.label}</span><span class="d num">· ${c.count} ${c.unit}</span></div><div class="faint" style="font-size:11.5px">${CAT_META[c.category][1]}</div></div>
      <span class="end"><span class="check ${on ? 'on' : ''}">${ic('check', 13, 2.4)}</span></span></div>`; }).join('')}</div>
    <div class="group-f num">${nSel} of ${all} settings selected · identical values are skipped</div>
  </div><div>
    <div class="group-t"><span>Targets</span><span class="num">${S.targets.length} of ${alts.length}</span></div>
    <div class="group">${alts.map(a => { const on = S.targets.includes(a.id); return `<div class="row click" data-act="target" data-id="${a.id}" style="min-height:48px">${av(a)}<div style="min-width:0"><div class="t">${esc(a.riotId)}</div><div class="faint" style="font-size:11.5px">${a.region} · ${esc(a.rank)} · synced ${ago(a.lastSynced).toLowerCase()}</div></div><span class="end"><span class="box ${on ? 'on' : ''}">${ic('check', 12, 2.6)}</span></span></div>`; }).join('')}</div>
    <div class="group-t" style="margin-top:20px"><span>Safety</span></div>
    <div class="group"><div class="row flat click" data-act="autobackup"><span class="t">Back up targets before overwrite</span><span class="end"><span class="switch ${S.autoBackup ? 'on' : ''}"></span></span></div>
    <div class="row flat click" data-act="skipsame"><span class="t">Skip identical settings</span><span class="end"><span class="switch ${S.skipSame ? 'on' : ''}"></span></span></div></div>
    <button class="btn pri lg" data-act="go" style="width:100%;margin-top:16px">${btn}</button>
  </div></div>${picker()}</section>`;
}

/* ---------- Diff ---------- */
function diffView() {
  const src = acc(S.source), tgt = acc(S.targets[0]), pend = pending();
  const per = c => pend.filter(e => e.category === c).length;
  const rows = S.diff.filter(e => (S.dcat === 'all' || e.category === S.dcat) && (!S.onlyChanged || e.kind !== 'same'));
  let last = '', html = '';
  for (const e of rows) {
    if (S.dcat === 'all' && e.category !== last) { last = e.category; html += `<div class="tr cat"><span></span><span>${CAT_META[e.category][0]}</span><span></span><span></span><span></span><span style="justify-self:end" class="num">${per(e.category)} of ${S.diff.filter(x => x.category === e.category).length}</span></div>`; }
    const k = keyOf(e), on = S.dsel.has(k), same = e.kind === 'same';
    const tag = { same: ['', 'Same'], changed: ['chg', 'Changed'], added: ['add', 'Added'], removed: ['rem', 'Removed'] }[e.kind];
    html += `<div class="tr ${same ? 'same' : 'chg'} ${!same && !on ? 'off' : ''} ${S.dhover === k ? 'hv' : ''}" ${same ? '' : `data-act="drow" data-id="${esc(k)}"`}>
      ${same ? '<span></span>' : `<span class="box ${on ? 'on' : ''}">${ic('check', 12, 2.6)}</span>`}<span class="k">${esc(pretty(e.key))}</span>
      <span class="v v-src">${fmtVal(e.category, e.source)}</span><span class="arr">${same ? '' : ic('arrow', 14, 1.8)}</span><span class="v v-tgt">${fmtVal(e.category, e.target)}</span><span class="tag ${tag[0]}">${tag[1]}</span></div>`;
  }
  return `<section class="screen" id="s-diff">
  <div class="head"><div><h1>Review changes</h1><p>Tick only the settings you want. Everything else on the target stays exactly as it is.</p></div><div class="sp"></div>
  <div class="pair"><span class="chip">${av(src)}${esc(src.riotId)}</span><span class="faint">${ic('arrow', 16)}</span><span class="chip">${av(tgt)}${esc(tgt.riotId)}</span><button class="btn" style="height:28px;padding:0 9px" data-act="swap" title="Swap">${ic('swap', 15)}</button></div></div>
  <div class="toolbar"><div class="seg">${[['all', 'All']].concat(S.cats.map(c => [c.category, c.label])).map(([id, l]) => `<button class="${S.dcat === id ? 'on' : ''}" data-act="dcat" data-id="${id}">${l}<span class="badge ${id !== 'all' && !per(id) ? 'gray' : ''}">${id === 'all' ? pend.length : per(id)}</span></button>`).join('')}</div>
  <div class="sp" style="flex:1"></div><div class="mini-sw click" data-act="only">Changed only<span class="switch ${S.onlyChanged ? 'on' : ''}"></span></div></div>
  <div class="tbl"><div class="tr th"><span></span><span>Setting</span><span>${esc(src.riotId)}</span><span></span><span>${esc(tgt.riotId)}</span><span style="justify-self:end">Status</span></div>${html}</div>
  </section><div class="applybar"><span class="sum num">${S.dsel.size} of ${pend.length} changes selected</span><span class="faint">${ic('shield', 15)}</span><span class="dim">${esc(tgt.riotId)} is backed up first</span><span class="sp"></span>
  <button class="btn ghost" data-act="dall">${S.dsel.size === pend.length ? 'Select none' : 'Select all'}</button><button class="btn pri" data-act="apply">Apply ${S.dsel.size} change${S.dsel.size === 1 ? '' : 's'}</button></div>`;
}

/* ---------- Crosshairs ---------- */
function scene() {
  return `<svg class="scene" width="100%" height="100%" viewBox="0 0 600 330" preserveAspectRatio="xMidYMid slice">
    <rect width="600" height="200" fill="#3B3F48"/><rect y="200" width="600" height="130" fill="#2A2C32"/>
    <rect x="0" y="60" width="150" height="140" fill="#454A55"/><rect x="40" y="120" width="70" height="80" fill="#363A43"/>
    <rect x="420" y="40" width="180" height="160" fill="#4A4F5B"/><rect x="455" y="95" width="56" height="105" fill="#30333B"/>
    <rect x="245" y="150" width="110" height="50" fill="#50555F"/><rect x="255" y="160" width="90" height="4" fill="#5E6470"/>
    <path d="M0 200L600 200" stroke="#1F2126" stroke-width="2"/><path d="M300 200L120 330M300 200L480 330" stroke="#33363D" stroke-width="1.5"/>
    <rect x="170" y="176" width="42" height="24" fill="#5A5F6B"/><rect x="380" y="168" width="30" height="32" fill="#41454F"/></svg>`;
}
function crosshairs() {
  const tgt = acc(S.targets[0]);
  if (S.code === null) S.code = S.xhs[S.xsel] ? S.xhs[S.xsel].code : '0;P';
  let p, err = null;
  try { p = parseXh(S.code); S.lastXh = p; } catch (e) { err = e; p = S.lastXh || parseXh('0;P'); }
  const yes = b => b ? 'On' : 'Off';
  const prm = (t, v) => `<div class="row prm flat"><span class="t">${t}</span><span class="end">${v}</span></div>`;
  const lines = (L, name) => `<div class="group-t" style="margin-top:18px"><span>${name}</span></div><div class="group">${prm('Show', yes(L.show))}${L.show ? prm('Length', trim(L.l)) + prm('Thickness', trim(L.t)) + prm('Offset', trim(L.o)) + prm('Opacity', (+L.a).toFixed(2)) + prm('Firing error', yes(L.f)) : ''}</div>`;
  return `<section class="screen" id="s-crosshairs">
  <div class="head"><div><h1>Crosshairs</h1><p>Paste any crosshair code to preview it, then save it to one or all of your accounts.</p></div><div class="sp"></div>
  <button class="btn">${ic('copy', 16)}Copy code</button><button class="btn pri">Save to ${esc(tgt.riotId)}</button></div>
  <div class="xh"><div><div class="group-t"><span>${esc(acc(S.source).riotId)}</span><span class="num">${S.xhs.length}</span></div><div class="group">
    ${S.xhs.map((x, i) => { const q = parseXh(x.code); return `<div class="row pick click ${i === S.xsel ? 'on' : ''}" data-act="xsel" data-id="${i}"><span class="xh-tile">${xhSvg(q, 1.5, 44)}</span><div style="min-width:0"><span class="t">${esc(x.name)}</span><span class="s">${PNAMES[q.color] || 'Custom ' + xhHex(q)} · inner ${q.inner.show ? trim(q.inner.l) + '/' + trim(q.inner.t) + '/' + trim(q.inner.o) : 'off'}</span></div></div>`; }).join('')}
    </div><div class="group-f">Codes are read from the vault snapshot. Shared codes are never uploaded anywhere.</div></div>
  <div><div class="stage">${scene()}<div class="xh-big">${xhSvg(p, S.zoom, 200)}</div>
    <div class="ov"><div class="seg">${[1, 2, 3].map(z => `<button class="${S.zoom === z ? 'on' : ''}" data-act="zoom" data-id="${z}">${z === 1 ? '1080p' : z + '×'}</button>`).join('')}</div><span class="badge gray" style="height:24px;padding:0 9px;background:rgba(15,16,19,.72)">${xhHex(p)}</span></div></div>
    <div class="codebox"><div class="lab"><span>Crosshair code</span>${err ? `<span class="err" ${S.focus ? 'style="color:var(--text-3)"' : ''}>${esc(err)}</span>` : `<span class="ok">${ic('check', 13, 2.2)}Valid · ${p.n} values</span>`}</div>
    <div class="code-in"><div class="field ${S.focus ? 'focus' : ''}">${S.code ? esc(S.code) : '<span class="faint">Paste a code, e.g. 0;P;c;5;h;0;0l;4</span>'}${S.focus ? '<span class="caret"></span>' : ''}</div><button class="btn" data-act="paste">${ic('paste', 16)}Paste</button></div></div>
    <div class="group-t" style="margin-top:18px"><span>Recently pasted</span></div><div class="group recent">${RECENT.map(([c, t]) => `<div class="row click" data-act="recent" data-id="${esc(c)}"><span class="xh-tile">${xhSvg(parseXh(c), 1, 30)}</span><span class="mono" style="flex:1;min-width:0">${esc(c)}</span><span class="faint" style="font-size:11.5px">${t}</span></div>`).join('')}</div></div>
  <div><div class="group-t"><span>Primary</span></div><div class="group">
    ${prm('Color', `<span class="sw-color" style="background:${xhHex(p)}"></span>${p.color === 8 ? 'Custom' : PNAMES[p.color]}`)}${prm('Outlines', p.outlines ? `On · ${trim(p.ot)} / ${(+p.oo).toFixed(2)}` : 'Off')}${prm('Center dot', p.dot ? `On · ${trim(p.dz)}` : 'Off')}</div>
    ${lines(p.inner, 'Inner lines')}${lines(p.outer, 'Outer lines')}</div></div></section>`;
}

/* ---------- Backups ---------- */
function backups() {
  const list = S.backups.filter(b => S.bkFilter === 'all' || (S.bkFilter === 'auto' ? b.kind === 'auto-backup' : b.kind !== 'auto-backup'));
  const b = S.backups.find(x => x.id === S.bk) || list[0], a = acc(b.accountId);
  const start = new Date(S.now * 1000).setHours(0, 0, 0, 0) / 1000;
  const day = t => { const d = t >= start ? 0 : Math.floor((start - t) / 86400) + 1; return d <= 0 ? 'Today' : d === 1 ? 'Yesterday' : new Date(t * 1000).toLocaleDateString('en-US', { weekday: 'long', month: 'short', day: 'numeric' }); };
  let cur = '', html = '';
  list.forEach((x, i) => {
    const d = day(x.createdAt), xa = acc(x.accountId);
    if (d !== cur) { cur = d; html += `<div class="day">${d}</div>`; }
    const nx = list[i + 1];
    html += `<div class="ev ${x.id === b.id ? 'on' : ''} ${i === 0 || day(list[i - 1].createdAt) !== d ? 'first' : ''} ${!nx || day(nx.createdAt) !== d ? 'last' : ''}" data-act="bk" data-id="${x.id}">
      <div class="rail"><span class="knob ${x.kind === 'auto-backup' ? 'auto' : ''}">${ic(x.kind, 13, 1.8)}</span></div>
      <div class="card"><span class="time">${hm(x.createdAt)}</span><span class="lbl">${esc(x.label)}</span><span class="acc">${av(xa)}${esc(xa.riotId)}</span><span class="sub num">${{ 'auto-backup': 'Auto-backup', manual: 'Snapshot', applied: 'Applied', imported: 'Imported' }[x.kind]} · ${x.total} settings · ${kb(x.bytes)}</span></div></div>`;
  });
  const max = Math.max(...Object.values(b.counts));
  return `<section class="screen" id="s-backups">
  <div class="head"><div><h1>Backups</h1><p>Every overwrite starts with an automatic snapshot. The last 20 per account are kept.</p></div><div class="sp"></div>
  <div class="seg">${[['all', 'All'], ['auto', 'Automatic'], ['manual', 'Snapshots']].map(([id, l]) => `<button class="${S.bkFilter === id ? 'on' : ''}" data-act="bkf" data-id="${id}">${l}</button>`).join('')}</div>
  <button class="btn pri" data-act="snap">${ic('plus', 15, 2)}New snapshot</button></div>
  <div class="bk"><div>${html}</div>
  <aside class="detail"><div class="faint" style="font-size:11px;font-weight:600;letter-spacing:.06em;text-transform:uppercase;margin-bottom:6px">${{ 'auto-backup': 'Automatic backup', manual: 'Manual snapshot', applied: 'Applied state', imported: 'Imported' }[b.kind]}</div>
    <h3>${esc(b.label)}</h3>
    <dl class="kv"><dt>Account</dt><dd>${esc(a.riotId)}</dd><dt>Created</dt><dd class="num">${new Date(b.createdAt * 1000).toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' })} · ${hm(b.createdAt)}</dd><dt>Settings</dt><dd class="num">${b.total} across ${Object.keys(b.counts).length} categories</dd><dt>Size</dt><dd class="num">${kb(b.bytes)}</dd></dl>
    <div class="bars">${S.cats.map(c => `<div class="bar"><span>${c.label}</span><i><b style="width:${(b.counts[c.category] || 0) / max * 100}%"></b></i><span class="num">${b.counts[c.category] || 0}</span></div>`).join('')}</div>
    <div class="path">${esc(S.vaultPath)}\\vault\\${esc(b.accountId)}\\${b.createdAt}-${esc(b.id)}.json</div>
    <div style="display:flex;gap:8px"><button class="btn pri" style="flex:1" data-act="restore">${ic('backups', 16)}Restore</button><button class="btn" data-act="reveal">${ic('folder', 16)}Show file</button></div>
    <div class="note">${ic('info', 16)}<span>Restoring backs up the current state of ${esc(a.riotId)} first, so it can be undone from this timeline.</span></div></aside></div></section>`;
}

/* ---------- Render + events ---------- */
const SCREENS = { transfer, diff: diffView, crosshairs, backups };
function render() {
  $('#sidebar').innerHTML = sidebar();
  $('#main').innerHTML = SCREENS[S.screen]();
  const r = S.riot;
  $('#tb-status').innerHTML = `<span class="dot ${r.clientRunning ? '' : 'off'}"></span>${r.clientRunning ? `Riot Client running${r.apiPort ? ' · local API port ' + r.apiPort : ''}` : 'Riot Client not running'} · VALORANT ${r.gameRunning ? 'running, close it before applying' : 'closed'}`;
  const pop = $('#pop');
  if (pop) {
    const card = document.querySelector(`[data-card="${S.picker}"]`).getBoundingClientRect(), main = $('#main').getBoundingClientRect();
    pop.style.left = (card.left - main.left + (S.picker === 'to' ? card.width - 300 : 0)) + 'px';
    pop.style.top = (card.bottom - main.top + 8) + 'px';
  }
  $('.tb-logo').innerHTML = '<svg width="18" height="18" viewBox="12 12 232 232"><rect x="12" y="12" width="232" height="232" rx="58" fill="#24252B"/><circle cx="72" cy="128" r="30" fill="none" stroke="#F2F2F5" stroke-width="14"/><circle cx="184" cy="128" r="38" fill="#8E7CFF"/><path d="M166 129l12 12 24-25" fill="none" stroke="#fff" stroke-width="13" stroke-linecap="round" stroke-linejoin="round"/></svg>';
}
function toast(msg) {
  const t = $('#toast');
  t.innerHTML = `<span style="color:var(--accent-2)">${ic('check', 16, 2)}</span>${esc(msg)}`;
  t.classList.add('show');
  clearTimeout(toast.h); toast.h = setTimeout(() => t.classList.remove('show'), 2600);
}
const toggle = (set, v) => set.has(v) ? set.delete(v) : set.add(v);

async function runTransfer() {
  if (S.tx.phase === 'run') return;
  if (S.tx.phase === 'done') { S.tx = { phase: 'idle', p: 0 }; return render(); }
  const call = tauri ? invoke('start_transfer', { plan: { sourceId: S.source, targetIds: S.targets, categories: [...S.sel], autoBackup: S.autoBackup } }) : Promise.resolve(null);
  const t0 = performance.now();
  S.tx = { phase: 'run', p: 0 };
  await new Promise(done => { const step = () => { S.tx.p = Math.min(1, (performance.now() - t0) / 2400); render(); S.tx.p < 1 ? requestAnimationFrame(step) : done(); }; step(); });
  try {
    const batch = await call;
    const failed = batch ? batch.jobs.filter(j => j.status === 'failed') : [];
    S.tx = { phase: 'done', p: 1 };
    S.targets.forEach(id => { acc(id).lastSynced = S.now; });
    render();
    toast(failed.length ? `${failed.length} account(s) failed: ${failed[0].error}` : `Transferred to ${S.targets.length} account${S.targets.length > 1 ? 's' : ''} · backups saved`);
  } catch (e) { S.tx = { phase: 'idle', p: 0 }; render(); toast(String(e)); }
}

document.addEventListener('click', async ev => {
  const win = ev.target.closest('[data-win]');
  if (win && tauri) return tauri.window.getCurrentWindow()[win.dataset.win]();
  const el = ev.target.closest('[data-act]');
  if (!el) { if (S.picker && !ev.target.closest('#pop')) { S.picker = null; render(); } return; }
  const id = el.dataset.id;
  switch (el.dataset.act) {
    case 'cat': toggle(S.sel, id); break;
    case 'allcats': S.sel = S.sel.size === S.cats.length ? new Set() : new Set(S.cats.map(c => c.category)); break;
    case 'target': if (S.targets.includes(id)) { if (S.targets.length > 1) S.targets = S.targets.filter(x => x !== id); } else S.targets.push(id); S.tx = { phase: 'idle', p: 0 }; break;
    case 'pick': S.picker = S.picker === id ? null : id; break;
    case 'choose':
      if (S.picker === 'from') { S.source = id; S.targets = S.targets.filter(x => x !== id); if (!S.targets.length) S.targets = [S.accounts.find(a => a.id !== id).id]; }
      else S.targets = [id];
      S.picker = null; S.tx = { phase: 'idle', p: 0 }; await loadDiff(); break;
    case 'autobackup': S.autoBackup = !S.autoBackup; break;
    case 'skipsame': S.skipSame = !S.skipSame; break;
    case 'go': return runTransfer();
    case 'dcat': S.dcat = id; break;
    case 'only': S.onlyChanged = !S.onlyChanged; break;
    case 'drow': toggle(S.dsel, id); break;
    case 'dall': S.dsel = S.dsel.size === pending().length ? new Set() : new Set(pending().map(keyOf)); break;
    case 'swap': { const t = S.targets[0]; S.targets = [S.source]; S.source = t; await loadDiff(); break; }
    case 'apply': {
      const keys = pending().filter(e => S.dsel.has(keyOf(e))).map(e => ({ category: e.category, key: e.key }));
      if (!keys.length) return;
      try {
        const r = tauri ? await invoke('apply_selected', { sourceId: S.source, targetId: S.targets[0], keys }) : { changed: keys.length };
        toast(`Applied ${r.changed} settings to ${acc(S.targets[0]).riotId}`); await loadDiff();
      } catch (e) { toast(String(e)); }
      break;
    }
    case 'xsel': S.xsel = +id; S.code = S.xhs[S.xsel].code; break;
    case 'zoom': S.zoom = +id; break;
    case 'recent': S.code = id; break;
    case 'paste': try { S.code = (await navigator.clipboard.readText()).trim() || S.code; S.focus = true; } catch (e) { toast('Clipboard is not available'); } break;
    case 'bk': S.bk = id; break;
    case 'bkf': S.bkFilter = id; break;
    case 'snap':
      if (tauri) { await invoke('create_snapshot', { accountId: S.source, label: 'Manual snapshot' }); await loadBackups(); }
      toast(`Snapshot of ${acc(S.source).riotId} saved`); break;
    case 'restore':
      try { if (tauri) { await invoke('restore_backup', { snapshotId: S.bk }); await loadBackups(); } toast('Restored · previous state backed up'); } catch (e) { toast(String(e)); }
      break;
    default: return;
  }
  render();
});
window.addEventListener('hashchange', () => { S.screen = SCREENS[location.hash.slice(1)] ? location.hash.slice(1) : 'transfer'; S.picker = null; render(); });

/* ---------- Tauri data loading ---------- */
async function loadDiff() {
  if (!tauri) return;
  const r = await invoke('compute_diff', { sourceId: S.source, targetId: S.targets[0], categories: [] });
  S.diff = r.entries; S.dsel = new Set(pending().map(keyOf));
}
async function loadBackups() {
  if (!tauri) return;
  S.backups = await invoke('list_backups', { accountId: null });
  if (S.backups[0]) S.bk = S.backups[0].id;
}
async function init() {
  S.screen = SCREENS[location.hash.slice(1)] ? location.hash.slice(1) : 'transfer';
  if (tauri) {
    try {
      const o = await invoke('get_overview');
      Object.assign(S, { accounts: o.accounts, cats: o.categories, vaultPath: o.vaultPath, riot: o.riot, now: Date.now() / 1000 });
      S.source = (o.accounts.find(a => a.isMain) || o.accounts[0]).id;
      S.targets = [o.accounts.find(a => a.id !== S.source).id];
      S.xhs = await invoke('list_crosshairs', { accountId: S.source });
      await loadDiff(); await loadBackups();
      setInterval(async () => { S.riot = await invoke('riot_status'); render(); }, 5000);
    } catch (e) { console.error(e); }
  }
  render();
}
window.VS = { S, render, parseXh };
init();
