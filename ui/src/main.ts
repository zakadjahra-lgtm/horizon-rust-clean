// Horizon Rust Clean — الواجهة. كل المنطق الحقيقي في Rust؛ هنا عرض واستدعاء أوامر فقط.
// لا يوجد innerHTML بنصوص قادمة من المستخدم: كل شيء يُبنى بـ DOM + textContent (حماية XSS).

interface Instance {
  id: string; name: string; version_id: string; max_memory_mb: number;
  java_path: string | null; installed: boolean; created_unix: number;
}
interface VersionInfo { id: string; kind: string }
interface Settings { username: string; default_ram_mb: number; java_path: string | null }
interface Prog { phase: string; done: number; total: number; files_done: number; files_total: number; speed: number }
interface ConsoleLine { id: string; err: boolean; line: string; sys?: boolean }
interface JavaInfo { installed: string[]; manual: string | null; data_dir: string }

const T = (window as any).__TAURI__;
const invoke = (cmd: string, args?: Record<string, unknown>): Promise<any> => T.core.invoke(cmd, args);
const listen = (ev: string, cb: (payload: any) => void): void => {
  T.event.listen(ev, (e: any) => cb(e.payload));
};

let instances: Instance[] = [];
let versions: VersionInfo[] = [];
let settings: Settings = { username: 'Player', default_ram_mb: 2048, java_path: null };
let page = 'home';
let selected: string | null = null;
const running = new Set<string>();
const installing = new Set<string>();
let prog: Prog | null = null;
const logLines: ConsoleLine[] = [];
const view = document.getElementById('view') as HTMLElement;

try { selected = localStorage.getItem('hrc.selected'); } catch { /* غير متاح */ }
function saveSel(): void { try { if (selected) localStorage.setItem('hrc.selected', selected); } catch { /* */ } }

// ───────────── أدوات DOM ─────────────
function h(tag: string, props: Record<string, any> = {}, ...kids: any[]): HTMLElement {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (v === false || v == null) continue;
    if (k === 'class') el.className = v;
    else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
    else if (v === true) el.setAttribute(k, '');
    else el.setAttribute(k, String(v));
  }
  for (const c of kids) { if (c != null && c !== false) el.append(c); }
  return el;
}

function toast(msg: string, isErr = false): void {
  const t = h('div', { class: 'toast' + (isErr ? ' err' : '') }, msg);
  document.getElementById('toasts')!.append(t);
  setTimeout(() => t.remove(), isErr ? 8000 : 3500);
}

function sys(msg: string): void {
  const l: ConsoleLine = { id: '', err: false, line: '[launcher] ' + msg, sys: true };
  pushLog(l);
}

function errMsg(e: unknown): string { return typeof e === 'string' ? e : String((e as any)?.message ?? e); }

const fmtBytes = (n: number): string =>
  n >= 1073741824 ? (n / 1073741824).toFixed(2) + ' GB' : n >= 1048576 ? (n / 1048576).toFixed(1) + ' MB' : (n / 1024).toFixed(0) + ' KB';

function openModal(title: string, content: HTMLElement[], okLabel: string, getValue: () => any): Promise<any> {
  return new Promise((resolve) => {
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') done(null);
      if (e.key === 'Enter') done(getValue());
    };
    const done = (v: any): void => { overlay.remove(); document.removeEventListener('keydown', onKey); resolve(v); };
    const overlay = h('div', { class: 'overlay' },
      h('div', { class: 'modal' }, h('h3', {}, title), ...content,
        h('div', { class: 'row end' },
          h('button', { class: 'btn', onclick: () => done(null) }, 'Cancel'),
          h('button', { class: 'btn primary', onclick: () => done(getValue()) }, okLabel))));
    document.addEventListener('keydown', onKey);
    document.body.append(overlay);
    const inp = overlay.querySelector('input');
    if (inp) (inp as HTMLInputElement).focus();
  });
}

async function askText(title: string, initial: string, type = 'text'): Promise<string | null> {
  const input = h('input', { class: 'input', type, value: initial }) as HTMLInputElement;
  return openModal(title, [input], 'OK', () => input.value);
}

async function askConfirm(title: string, msg: string): Promise<boolean> {
  return (await openModal(title, [h('p', { class: 'muted' }, msg)], 'Confirm', () => true)) === true;
}

// ───────────── البيانات ─────────────
function current(): Instance | undefined {
  return instances.find((i) => i.id === selected) ?? instances[0];
}

async function refreshInstances(): Promise<void> {
  instances = await invoke('list_instances');
  if (!instances.find((i) => i.id === selected)) selected = instances[0]?.id ?? null;
}

async function ensureVersions(): Promise<boolean> {
  if (versions.length > 0) return true;
  try { versions = await invoke('list_versions'); return true; }
  catch (e) { toast('Could not load versions: ' + errMsg(e), true); return false; }
}

// ───────────── التقدّم ─────────────
function progressBlock(): HTMLElement {
  return h('div', {},
    h('div', { class: 'bar' }, h('div', { id: 'pbar' })),
    h('div', { class: 'muted', id: 'ptext', style: 'margin-top:6px' }, 'Idle'));
}

function updateProgress(): void {
  const bar = document.getElementById('pbar');
  const txt = document.getElementById('ptext');
  if (!bar || !txt) return;
  if (!prog || installing.size === 0) { bar.style.width = '0%'; txt.textContent = 'Idle'; return; }
  const pct = prog.total > 0 ? Math.min(100, (prog.done / prog.total) * 100) : 0;
  bar.style.width = pct.toFixed(1) + '%';
  const what = prog.phase === 'java' ? 'Java runtime' : 'Game files';
  txt.textContent = `${what}: ${pct.toFixed(0)}% — ${fmtBytes(prog.done)} / ${fmtBytes(prog.total)} — ${fmtBytes(prog.speed)}/s — ${prog.files_done}/${prog.files_total} files`;
}

// ───────────── الصفحات ─────────────
async function startPlay(id: string): Promise<void> {
  const inst = instances.find((i) => i.id === id);
  if (!inst) return;
  try {
    if (!inst.installed) {
      installing.add(id); sys(`Installing ${inst.version_id}…`); render();
      try { await invoke('install_instance', { id }); } finally { installing.delete(id); }
      await refreshInstances();
      sys(`Installed ${inst.version_id}`);
    }
    await invoke('launch_instance', { id });
  } catch (e) {
    installing.delete(id);
    toast(errMsg(e), true); sys('Error: ' + errMsg(e));
  }
  render();
}

function renderHome(): HTMLElement {
  const wrap = h('div', {}, h('h1', {}, 'Play'));
  const inst = current();
  if (!inst) {
    wrap.append(h('div', { class: 'card center' },
      h('h2', {}, 'Welcome to Horizon Rust Clean'),
      h('p', { class: 'muted' }, 'Create an instance (a Minecraft version with its own folder and settings) to get started.'),
      h('button', { class: 'btn primary', onclick: () => go('instances') }, 'Create instance')));
    return wrap;
  }
  const sel = h('select', {
    class: 'input',
    onchange: (e: Event) => { selected = (e.target as HTMLSelectElement).value; saveSel(); render(); },
  }, ...instances.map((i) => h('option', { value: i.id }, `${i.name} — ${i.version_id}`))) as HTMLSelectElement;
  sel.value = inst.id;
  const isRun = running.has(inst.id), isInst = installing.has(inst.id);
  const label = isRun ? 'RUNNING' : isInst ? 'INSTALLING…' : inst.installed ? 'PLAY' : 'INSTALL & PLAY';
  wrap.append(h('div', { class: 'card' },
    h('div', { class: 'row' }, sel,
      h('span', { class: 'badge ' + (inst.installed ? 'ok' : '') }, inst.installed ? 'installed' : 'not installed')),
    h('p', { class: 'muted' }, `Minecraft ${inst.version_id} · ${inst.max_memory_mb} MB RAM · ${settings.username} (offline)`),
    h('button', { class: 'play', disabled: isRun || isInst, onclick: () => { void startPlay(inst.id); } }, label),
    progressBlock()));
  return wrap;
}

function kindLabel(k: string): string {
  return k === 'release' ? 'Release' : k === 'snapshot' ? 'Snapshot' : k === 'old_beta' ? 'Beta' : k === 'old_alpha' ? 'Alpha' : k;
}

function renderInstances(): HTMLElement {
  const wrap = h('div', {}, h('h1', {}, 'Instances'));

  // نموذج الإنشاء
  const name = h('input', { class: 'input', value: 'My instance', style: 'width:200px' }) as HTMLInputElement;
  const ram = h('input', { class: 'input', type: 'number', min: 512, step: 256, value: settings.default_ram_mb, style: 'width:110px' }) as HTMLInputElement;
  const verSel = h('select', { class: 'input' }, h('option', { value: '' }, 'Loading versions…')) as HTMLSelectElement;
  const kinds: Record<string, boolean> = { release: true, snapshot: false, old_beta: false, old_alpha: false };
  const fill = (): void => {
    verSel.replaceChildren(...versions.filter((v) => kinds[v.kind])
      .map((v) => h('option', { value: v.id }, `${v.id}  (${kindLabel(v.kind)})`)));
  };
  const chks = Object.keys(kinds).map((k) => h('label', { class: 'chk' },
    h('input', { type: 'checkbox', checked: kinds[k], onchange: (e: Event) => { kinds[k] = (e.target as HTMLInputElement).checked; fill(); } }),
    kindLabel(k)));
  void ensureVersions().then((ok) => { if (ok) fill(); else verSel.replaceChildren(h('option', { value: '' }, 'Offline — retry later')); });

  wrap.append(h('div', { class: 'card' }, h('h2', {}, 'New instance'),
    h('div', { class: 'row' }, name, verSel, ram, h('span', { class: 'muted' }, 'MB'),
      h('button', {
        class: 'btn primary',
        onclick: async () => {
          try {
            const created: Instance = await invoke('create_instance', { name: name.value, versionId: verSel.value, maxMemoryMb: Number(ram.value) || 2048 });
            await refreshInstances(); selected = created.id; saveSel(); toast('Instance created'); render();
          } catch (e) { toast(errMsg(e), true); }
        },
      }, 'Create')),
    h('div', { class: 'row', style: 'margin-top:10px' }, ...chks),
    h('p', { class: 'muted', style: 'margin:8px 0 0' }, 'Beta/Alpha versions are old and may not run on modern systems.')));

  for (const i of instances) wrap.append(instanceCard(i));
  return wrap;
}

function instanceCard(i: Instance): HTMLElement {
  const busy = running.has(i.id) || installing.has(i.id);
  const act = (label: string, fn: () => Promise<void>, cls = ''): HTMLElement =>
    h('button', {
      class: 'btn ' + cls, disabled: busy,
      onclick: async () => { try { await fn(); } catch (e) { toast(errMsg(e), true); } },
    }, label);
  return h('div', { class: 'card' },
    h('div', { class: 'list-item' },
      h('div', { class: 'grow' },
        h('h2', { style: 'margin:0' }, i.name),
        h('div', { class: 'muted' }, `Minecraft ${i.version_id} · ${i.max_memory_mb} MB RAM `,
          h('span', { class: 'badge ' + (i.installed ? 'ok' : '') }, i.installed ? 'installed' : 'not installed'))),
      h('div', { class: 'row' },
        act('Select', async () => { selected = i.id; saveSel(); go('home'); }),
        act('Rename', async () => {
          const n = await askText('Rename instance', i.name);
          if (n !== null) { await invoke('rename_instance', { id: i.id, name: n }); await refreshInstances(); render(); }
        }),
        act('RAM', async () => {
          const v = await askText('Max RAM (MB)', String(i.max_memory_mb), 'number');
          if (v !== null) { await invoke('set_instance_ram', { id: i.id, mb: Number(v) || 2048 }); await refreshInstances(); render(); }
        }),
        act('Duplicate', async () => { await invoke('duplicate_instance', { id: i.id }); await refreshInstances(); toast('Duplicated'); render(); }),
        act(i.installed ? 'Reinstall' : 'Install', async () => {
          installing.add(i.id); go('downloads');
          try { await invoke('install_instance', { id: i.id }); sys('Installed ' + i.version_id); toast('Installed'); }
          finally { installing.delete(i.id); await refreshInstances(); render(); }
        }),
        act('Open folder', async () => { await invoke('open_instance_folder', { id: i.id }); }),
        act('Delete', async () => {
          if (await askConfirm('Delete instance', `Delete "${i.name}" with its worlds and settings? Shared game files are kept.`)) {
            await invoke('delete_instance', { id: i.id }); await refreshInstances(); render();
          }
        }, 'danger'))));
}

function renderDownloads(): HTMLElement {
  return h('div', {}, h('h1', {}, 'Downloads'),
    h('div', { class: 'card' }, progressBlock(),
      h('p', { class: 'muted' }, 'Files are verified with SHA1 and corrupted files are re-downloaded automatically. Interrupted downloads resume where they stopped.')));
}

function renderJava(): HTMLElement {
  const wrap = h('div', {}, h('h1', {}, 'Java'));
  const list = h('div', { class: 'muted' }, 'Loading…');
  const manual = h('input', { class: 'input', style: 'width:100%', placeholder: 'Leave empty to use the official runtime automatically' }) as HTMLInputElement;
  void invoke('java_info').then((j: JavaInfo) => {
    list.replaceChildren(j.installed.length === 0
      ? h('span', {}, 'No runtime installed yet. The right one is downloaded automatically when you install an instance.')
      : h('div', {}, ...j.installed.map((n) => h('div', {}, '• ' + n))));
    manual.value = j.manual ?? settings.java_path ?? '';
  }).catch((e) => list.replaceChildren(errMsg(e)));
  wrap.append(
    h('div', { class: 'card' }, h('h2', {}, 'Installed runtimes (Mojang official)'), list),
    h('div', { class: 'card' }, h('h2', {}, 'Manual Java path (optional)'), manual,
      h('div', { class: 'row end' }, h('button', {
        class: 'btn primary',
        onclick: async () => {
          try {
            settings = { ...settings, java_path: manual.value.trim() || null };
            await invoke('save_settings', { settings }); toast('Saved');
          } catch (e) { toast(errMsg(e), true); }
        },
      }, 'Save'))));
  return wrap;
}

let consoleEl: HTMLElement | null = null;
function logSpan(l: ConsoleLine): HTMLElement {
  return h('div', { class: l.sys ? 'sys' : l.err ? 'err' : '' }, l.line);
}
function pushLog(l: ConsoleLine): void {
  logLines.push(l);
  if (logLines.length > 3000) logLines.shift();
  if (page === 'console' && consoleEl) {
    consoleEl.append(logSpan(l));
    consoleEl.scrollTop = consoleEl.scrollHeight;
  }
}

function renderConsole(): HTMLElement {
  consoleEl = h('div', { class: 'console' }, ...logLines.map(logSpan));
  const el = consoleEl;
  setTimeout(() => { el.scrollTop = el.scrollHeight; }, 0);
  return h('div', {}, h('h1', {}, 'Console'),
    h('div', { class: 'row', style: 'margin-bottom:10px' },
      h('button', { class: 'btn', onclick: () => { logLines.length = 0; render(); } }, 'Clear')),
    consoleEl);
}

function renderSettings(): HTMLElement {
  const user = h('input', { class: 'input', value: settings.username }) as HTMLInputElement;
  const ram = h('input', { class: 'input', type: 'number', min: 512, step: 256, value: settings.default_ram_mb }) as HTMLInputElement;
  const dataDir = h('span', { class: 'muted' }, '…');
  void invoke('java_info').then((j: JavaInfo) => { dataDir.textContent = j.data_dir; }).catch(() => { /* */ });
  return h('div', {}, h('h1', {}, 'Settings'),
    h('div', { class: 'card' },
      h('h2', {}, 'Player (offline mode)'),
      h('p', { class: 'muted' }, 'Microsoft sign-in is not available yet. Offline mode is for testing and does not work on online servers.'),
      user),
    h('div', { class: 'card' }, h('h2', {}, 'Default RAM for new instances (MB)'), ram),
    h('div', { class: 'card' }, h('h2', {}, 'Data folder'), h('div', { class: 'row' }, dataDir,
      h('button', { class: 'btn', onclick: () => { void invoke('open_data_folder'); } }, 'Open'))),
    h('div', { class: 'row' }, h('button', {
      class: 'btn primary',
      onclick: async () => {
        try {
          const next: Settings = { ...settings, username: user.value.trim(), default_ram_mb: Number(ram.value) || 2048 };
          await invoke('save_settings', { settings: next });
          settings = next; toast('Saved'); render();
        } catch (e) { toast(errMsg(e), true); }
      },
    }, 'Save')));
}

// ───────────── التنقل والتشغيل ─────────────
function render(): void {
  consoleEl = null;
  document.querySelectorAll('.nav button').forEach((b) =>
    b.classList.toggle('active', (b as HTMLElement).dataset.page === page));
  const pages: Record<string, () => HTMLElement> = {
    home: renderHome, instances: renderInstances, downloads: renderDownloads,
    java: renderJava, console: renderConsole, settings: renderSettings,
  };
  view.replaceChildren((pages[page] ?? renderHome)());
  updateProgress();
}

function go(p: string): void { page = p; render(); }

listen('progress', (p: Prog) => { prog = p; updateProgress(); });
listen('console', (c: ConsoleLine) => pushLog(c));
listen('game-started', (e: any) => { running.add(e.id); sys('Game started'); if (page === 'home' || page === 'instances') render(); });
listen('game-exit', (e: any) => {
  running.delete(e.id);
  sys(`Game exited (code ${e.code ?? 'unknown'})`);
  if (!e.ok) toast(`Game exited with code ${e.code ?? 'unknown'} — see Console`, true);
  if (page === 'home' || page === 'instances') render();
});

async function init(): Promise<void> {
  document.querySelectorAll('.nav button').forEach((b) =>
    b.addEventListener('click', () => go((b as HTMLElement).dataset.page ?? 'home')));
  try { settings = await invoke('get_settings'); } catch { /* الافتراضي */ }
  try { await refreshInstances(); } catch (e) { toast(errMsg(e), true); }
  go('home');
}
void init();
