// Horizon Rust Clean — الواجهة. كل المنطق الحقيقي في Rust؛ هنا عرض واستدعاء أوامر فقط.
// لا يوجد innerHTML بنصوص قادمة من المستخدم: كل شيء يُبنى بـ DOM + textContent (حماية XSS).

interface Instance {
  id: string; name: string; version_id: string; max_memory_mb: number;
  java_path: string | null; installed: boolean; created_unix: number;
  loader: string; loader_version: string | null; launch_id: string | null;
}
interface ContentItem { name: string; enabled: boolean; size: number; is_dir: boolean; warning: string | null }
interface ClientProfile {
  id: string; name: string; hud_scale: number; perf: string; rust_mods: Record<string, boolean>;
  mods: Record<string, { enabled: boolean; x: number; y: number }>;
}
interface VersionInfo { id: string; kind: string }
interface Settings { username: string; default_ram_mb: number; java_path: string | null; curseforge_key?: string | null }
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
// كل نوع بيئة = instances مستقلة بمجلداتها (مودات/حزم موارد/shaders) ولا تتشارك شيئاً.
const LOADERS: { id: string; label: string; hint: string; hidden?: boolean }[] = [
  { id: 'vanilla', label: 'Vanilla', hint: 'Original Minecraft, no mods. Resource packs work; shaders need a shader mod.' },
  { id: 'fabric', label: 'Fabric', hint: 'Lightweight mod loader (Minecraft 1.14 and newer). Mods stay inside this instance only.' },
  { id: 'quilt', label: 'Quilt', hint: 'Fabric-compatible mod loader (Minecraft 1.14 and newer).' },
  { id: 'forge', label: 'Forge', hint: 'Installed with Forge\'s official installer; the first install can take a few minutes.' },
  { id: 'neoforge', label: 'NeoForge', hint: 'NeoForge supports Minecraft 1.20.2 and newer.' },
  { id: 'rusttools', hidden: true, label: 'Horizon (Rust tools)', hint: 'Original Minecraft (Java), any version. No mods and nothing injected into the game. The launcher (Rust) adds an optional performance preset and tools: log cleaner, crash cleaner, screenshot sorter.' },
  { id: 'rust', hidden: true, label: 'Horizon Rust (experimental)', hint: 'A native game written in Rust — NOT Minecraft. No Java, no Minecraft versions, no mods. Fly over a generated block world. Early prototype.' },
  { id: 'horizon', label: 'Rust', hint: 'Our clean client. HUD modules work on 1.8–1.12.2; newer versions run on a Fabric base. All modules start OFF.' },
];
// الإدخالان المخفيان لا يظهران في القوائم؛ تبقى المعرّفات حتى لا تتعطل instances قديمة.
const SHOWN = LOADERS.filter((l) => !l.hidden);
const RUST_VERSION = 'horizon-rust-0.1';
const isRust = (i: Instance): boolean => i.loader === 'rust';
const loaderLabel = (id: string): string => LOADERS.find((l) => l.id === id)?.label ?? id;
const isLegacyMc = (v: string): boolean => /^1\.(8|9|10|11|12)(\.|$)/.test(v);
// Horizon على 1.13+ يعمل فوق Fabric فيقبل مودات Fabric؛ على 1.8–1.12.2 لا مودات خارجية.
const supportsMods = (i: Instance): boolean =>
  ['fabric', 'quilt', 'forge', 'neoforge'].includes(i.loader) || (i.loader === 'horizon' && !isLegacyMc(i.version_id));

let page = 'home';
let filterLoader = 'all';
let contentFor: string | null = null;
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
      h('h2', {}, 'Welcome to MClauncher'),
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
    h('p', { class: 'muted' }, (isRust(inst) ? `Native Rust game (not Minecraft) · ${settings.username}` : `Minecraft ${inst.version_id} · ${inst.max_memory_mb} MB RAM · ${settings.username} (offline)`)),
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
  const loaderSel = h('select', { class: 'input', style: 'min-width:200px' },
    ...SHOWN.map((l) => h('option', { value: l.id }, l.label))) as HTMLSelectElement;
  if (filterLoader !== 'all') loaderSel.value = filterLoader;
  const hint = h('p', { class: 'muted', style: 'margin:8px 0 0' }, '');
  const setHint = (): void => {
    hint.textContent = LOADERS.find((l) => l.id === loaderSel.value)?.hint ?? '';
    verSel.style.display = loaderSel.value === 'rust' ? 'none' : '';
  };
  loaderSel.addEventListener('change', setHint);
  setHint();
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
    h('div', { class: 'row' }, name, loaderSel, verSel, ram, h('span', { class: 'muted' }, 'MB'),
      h('button', {
        class: 'btn primary',
        onclick: async () => {
          try {
            const created: Instance = await invoke('create_instance', { name: name.value, versionId: loaderSel.value === 'rust' ? RUST_VERSION : verSel.value, maxMemoryMb: Number(ram.value) || 2048, loader: loaderSel.value });
            await refreshInstances(); selected = created.id; saveSel(); toast('Instance created'); render();
          } catch (e) { toast(errMsg(e), true); }
        },
      }, 'Create')),
    hint,
    h('div', { class: 'row', style: 'margin-top:10px' }, ...chks),
    h('p', { class: 'muted', style: 'margin:8px 0 0' }, 'Beta/Alpha versions are old and may not run on modern systems.')));

  // تبويبات: كل بيئة لها مكانها
  const tabs = h('div', { class: 'row', style: 'margin:6px 0 14px' },
    ...[{ id: 'all', label: 'All' }, ...SHOWN].map((l) => h('button', {
      class: 'btn' + (filterLoader === l.id ? ' primary' : ''),
      onclick: () => { filterLoader = l.id; render(); },
    }, l.label)));
  wrap.append(tabs);
  const shown = instances.filter((i) => filterLoader === 'all' || i.loader === filterLoader);
  if (shown.length === 0) wrap.append(h('p', { class: 'muted' }, 'No instances in this environment yet.'));
  for (const i of shown) wrap.append(instanceCard(i));
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
        h('div', { class: 'muted' }, (isRust(i) ? 'Native Rust game (not Minecraft) ' : `Minecraft ${i.version_id} · ${i.max_memory_mb} MB RAM `),
          h('span', { class: 'badge' }, loaderLabel(i.loader)), ' ',
          h('span', { class: 'badge ' + (i.installed ? 'ok' : '') }, i.installed ? 'installed' : 'not installed'))),
      h('div', { class: 'row' },
        act('Select', async () => { selected = i.id; saveSel(); go('home'); }),
        h('button', { class: 'btn', onclick: () => { contentFor = i.id; go('content'); } }, 'Content'),
        act('Rename', async () => {
          const n = await askText('Rename instance', i.name);
          if (n !== null) { await invoke('rename_instance', { id: i.id, name: n }); await refreshInstances(); render(); }
        }),
        act('RAM', async () => {
          const v = await askText('Max RAM (MB)', String(i.max_memory_mb), 'number');
          if (v !== null) { await invoke('set_instance_ram', { id: i.id, mb: Number(v) || 2048 }); await refreshInstances(); render(); }
        }),
        act('Duplicate', async () => { await invoke('duplicate_instance', { id: i.id }); await refreshInstances(); toast('Duplicated'); render(); }),
        act(i.installed ? 'Update / Reinstall' : 'Install', async () => {
          installing.add(i.id); go('downloads');
          try { await invoke('install_instance', { id: i.id }); sys('Installed ' + i.version_id); toast('Installed'); }
          finally { installing.delete(i.id); await refreshInstances(); render(); }
        }),
        act('Open folder', async () => { await invoke('open_instance_folder', { id: i.id }); }),
        act('Delete', async () => {
          if (await askConfirm('Delete instance', `Delete "${i.name}" with its worlds, mods and settings? Shared game files are kept.`)) {
            await invoke('delete_instance', { id: i.id }); await refreshInstances(); render();
          }
        }, 'danger'))));
}

// ───────────── صفحة Content: مودات / حزم موارد / shaders لكل instance ─────────────
function renderContent(): HTMLElement {
  const inst = instances.find((i) => i.id === contentFor);
  const wrap = h('div', {}, h('button', { class: 'btn', onclick: () => go('instances') }, '← Instances'));
  if (!inst) { wrap.append(h('p', { class: 'muted' }, 'Instance not found.')); return wrap; }
  wrap.append(h('h1', { style: 'margin-top:14px' }, inst.name),
    h('p', { class: 'muted' }, `${loaderLabel(inst.loader)}${isRust(inst) ? '' : ' · Minecraft ' + inst.version_id} — files here belong to this instance only.`));
  if (isRust(inst)) {
    wrap.append(h('div', { class: 'card' }, h('h2', {}, 'Horizon Rust (experimental)'),
      h('p', { class: 'muted' }, 'Controls: click the window to capture the mouse, W A S D to fly, Space / Left Shift up and down, Left Ctrl to sprint, Esc to release the mouse (Esc again to quit). Your position is saved in this instance\'s folder.'),
      h('p', { class: 'muted' }, 'Optional file in the instance folder: horizon-rust.properties with seed=…, radius=1..16, vsync=true.'),
      h('button', { class: 'btn', onclick: () => { void invoke('open_instance_folder', { id: inst.id }).catch((e) => toast(errMsg(e), true)); } }, 'Open folder')));
    return wrap;
  }
  if (inst.loader === 'horizon' || inst.loader === 'rusttools') wrap.append(clientPanel(inst));
  if (supportsMods(inst)) {
    wrap.append(modBrowser(inst));
    wrap.append(contentSection(inst, 'mods', 'Mods', 'The checkbox turns a mod on or off without deleting it. Changes apply the next time you launch.'));
  }
  wrap.append(contentSection(inst, 'resourcepacks', 'Resource packs', 'Enable them in game: Options → Resource Packs.'));
  wrap.append(contentSection(inst, 'shaderpacks', 'Shader packs', 'Shaders need a shader mod such as Iris (Fabric / Quilt / NeoForge) installed in this instance.'));
  return wrap;
}

function contentSection(inst: Instance, kind: string, title: string, note: string): HTMLElement {
  const list = h('div', {}, h('span', { class: 'muted' }, 'Loading…'));
  const row = (it: ContentItem): HTMLElement => h('div', { class: 'list-item', style: 'padding:6px 0;border-top:1px solid var(--line)' },
    kind === 'mods' ? h('input', {
      type: 'checkbox', checked: it.enabled,
      onchange: async (e: Event) => {
        try { await invoke('toggle_mod', { id: inst.id, name: it.name, enabled: (e.target as HTMLInputElement).checked }); }
        catch (err) { toast(errMsg(err), true); }
        await reload();
      },
    }) : null,
    h('span', { class: 'grow' }, it.name + (it.is_dir ? '  (folder)' : ''),
      it.warning ? h('div', { style: 'color:#ff6b6b;font-size:12px' }, '⚠ ' + it.warning) : null),
    h('span', { class: 'muted' }, it.is_dir ? '' : fmtBytes(it.size)),
    h('button', {
      class: 'btn danger',
      onclick: async () => {
        if (!(await askConfirm('Delete', `Delete "${it.name}"?`))) return;
        try { await invoke('delete_content', { id: inst.id, kind, name: it.name }); } catch (err) { toast(errMsg(err), true); }
        await reload();
      },
    }, 'Delete'));
  const reload = async (): Promise<void> => {
    try {
      const items: ContentItem[] = await invoke('list_content', { id: inst.id, kind });
      list.replaceChildren(...(items.length === 0
        ? [h('span', { class: 'muted' }, 'Nothing here yet. Use "Open folder" and drop files in.')]
        : items.map(row)));
    } catch (e) { list.replaceChildren(errMsg(e)); }
  };
  void reload();
  return h('div', { class: 'card' },
    h('div', { class: 'row' }, h('h2', { class: 'grow', style: 'margin:0' }, title),
      h('button', { class: 'btn', onclick: () => { void invoke('open_content_folder', { id: inst.id, kind }).catch((e) => toast(errMsg(e), true)); } }, 'Open folder')),
    h('p', { class: 'muted' }, note), list);
}

function modBrowser(inst: Instance): HTMLElement {
  const out = h('div', {}, h('span', { class: 'muted' }, 'Search Modrinth, or load the suggested light mods. Offline? Use "Open folder" and drop .jar files in.'));
  const q = h('input', { class: 'input', placeholder: 'Search mods…', style: 'width:240px' }) as HTMLInputElement;
  const addBtn = (slug: string): HTMLElement => h('button', {
    class: 'btn',
    onclick: async (e: Event) => {
      const b = e.target as HTMLButtonElement; b.disabled = true; b.textContent = 'Installing…';
      try {
        const got: string[] = await invoke('install_mod', { id: inst.id, project: slug });
        toast(got.length ? `Installed ${got.join(', ')}` : 'Already installed');
        b.textContent = 'Installed';
        render();
      } catch (err) { toast(errMsg(err), true); b.disabled = false; b.textContent = 'Install'; }
    },
  }, 'Install');
  const line = (slug: string, title: string, note: string): HTMLElement =>
    h('div', { class: 'list-item', style: 'padding:6px 0;border-top:1px solid var(--line)' },
      h('div', { class: 'grow' }, h('strong', {}, title), h('div', { class: 'muted' }, note)), addBtn(slug));
  const doSearch = async (): Promise<void> => {
    out.replaceChildren(h('span', { class: 'muted' }, 'Searching…'));
    try {
      const hits: { slug: string; title: string; description: string }[] = await invoke('search_mods', { id: inst.id, query: q.value });
      out.replaceChildren(...(hits.length ? hits.map((x) => line(x.slug, x.title, x.description)) : [h('span', { class: 'muted' }, 'No results for this Minecraft version.')]));
    } catch (e) { out.replaceChildren(h('span', { class: 'muted' }, 'Could not reach Modrinth (' + errMsg(e) + '). You can still add .jar files by hand.')); }
  };
  const showSuggested = async (): Promise<void> => {
    const list: { slug: string; note: string }[] = await invoke('curated_mods');
    out.replaceChildren(...list.map((x) => line(x.slug, x.slug, x.note)));
  };
  return h('div', { class: 'card' }, h('h2', {}, 'Get mods'),
    h('div', { class: 'row' }, q,
      h('button', { class: 'btn primary', onclick: () => { void doSearch(); } }, 'Search'),
      h('button', { class: 'btn', onclick: () => { void showSuggested(); } }, 'Suggested light mods')),
    out);
}

function clientPanel(inst: Instance): HTMLElement {
  const title = inst.loader === 'rusttools' ? 'Horizon (Rust tools)' : 'Rust';
  const hud = inst.loader === 'horizon';
  const box = h('div', { class: 'card' }, h('h2', {}, title), h('span', { class: 'muted' }, 'Loading…'));
  const legacy = isLegacyMc(inst.version_id);
  void invoke('get_client_profile', { id: inst.id }).then((p: ClientProfile) => {
    const rustLabels: Record<string, string> = {
      log_cleaner: 'Log cleaner — delete compressed logs older than 14 days',
      crash_cleaner: 'Crash cleaner — delete crash reports older than 30 days',
      screenshot_sorter: 'Screenshot sorter — group screenshots into dated folders',
    };
    const labels: Record<string, string> = { fps: 'FPS counter', cps: 'CPS counter', keystrokes: 'Keystrokes' };
    const rows = Object.keys(labels).map((k) => h('label', { class: 'chk', style: 'display:flex;margin:6px 0' },
      h('input', { type: 'checkbox', checked: p.mods[k]?.enabled, disabled: !legacy, onchange: (e: Event) => { p.mods[k].enabled = (e.target as HTMLInputElement).checked; } }),
      `${labels[k]} (beta)`));
    const perf = h('select', { class: 'input', onchange: (e: Event) => { p.perf = (e.target as HTMLSelectElement).value; } },
      ...['vanilla', 'balanced', 'performance'].map((v) => h('option', { value: v, selected: p.perf === v }, v)));
    box.replaceChildren(
      h('h2', {}, title),
      h('p', { class: 'muted' }, !hud ? 'Plain Minecraft with launcher-side tools only. Nothing is injected into the game; changes apply on the next launch.' : legacy
        ? 'These HUD modules run inside Minecraft 1.8–1.12.2. All are off by default; changes apply on the next launch.'
        : 'HUD modules are available for Minecraft 1.8–1.12.2 only for now. This version runs on a Fabric base; the performance preset still applies.'),
      ...(hud ? rows : []),
      h('h3', { style: 'margin:14px 0 4px' }, 'Rust mods (beta)'),
      h('p', { class: 'muted', style: 'margin:0 0 4px' }, 'Small tools that run in the launcher before the game starts. All off by default.'),
      ...Object.keys(rustLabels).map((k) => h('label', { class: 'chk', style: 'display:flex;margin:6px 0' },
        h('input', { type: 'checkbox', checked: !!p.rust_mods?.[k], onchange: (e: Event) => { p.rust_mods = p.rust_mods ?? {}; p.rust_mods[k] = (e.target as HTMLInputElement).checked; } }),
        rustLabels[k])),
      h('div', { class: 'row', style: 'margin-top:8px' }, h('span', { class: 'muted' }, 'Performance preset (JVM flags, not benchmarked):'), perf),
      h('div', { class: 'row end' }, h('button', {
        class: 'btn primary',
        onclick: async () => {
          try { await invoke('save_client_profile', { id: inst.id, profile: p }); toast('Saved'); }
          catch (e) { toast(errMsg(e), true); }
        },
      }, 'Save')));
  }).catch((e) => box.replaceChildren(h('h2', {}, title), h('p', { class: 'muted' }, errMsg(e))));
  return box;
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

function netCheckCard(): HTMLElement {
  const out = h('div', {});
  return h('div', { class: 'card' }, h('h2', {}, 'Network check'),
    h('p', { class: 'muted' }, 'Tests every official server the launcher needs and shows the real reason if one is unreachable (firewall, antivirus, VPN, DNS).'),
    h('button', {
      class: 'btn',
      onclick: async (e: Event) => {
        const b = e.target as HTMLButtonElement; b.disabled = true; out.replaceChildren(h('span', { class: 'muted' }, 'Testing…'));
        try {
          const r: { name: string; ok: boolean; detail: string }[] = await invoke('net_check');
          out.replaceChildren(...r.map((c) => h('div', { style: 'padding:3px 0;color:' + (c.ok ? '#6bdc8b' : '#ff6b6b') }, `${c.ok ? '✔' : '✘'} ${c.name} — ${c.detail}`)));
        } catch (err) { out.replaceChildren(errMsg(err)); }
        b.disabled = false;
      },
    }, 'Run check'), out);
}

function renderSettings(): HTMLElement {
  const user = h('input', { class: 'input', value: settings.username }) as HTMLInputElement;
  const ram = h('input', { class: 'input', type: 'number', min: 512, step: 256, value: settings.default_ram_mb }) as HTMLInputElement;
  const cfKey = h('input', { class: 'input', type: 'password', placeholder: 'API key', value: settings.curseforge_key ?? '', style: 'width:320px' }) as HTMLInputElement;
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
    h('div', { class: 'card' }, h('h2', {}, 'CurseForge API key (optional)'),
      h('p', { class: 'muted' }, 'Get a free key at console.curseforge.com. It is kept only in this computer\'s settings file and is used for CurseForge searches in Browse.'),
      cfKey),
    netCheckCard(),
    h('div', { class: 'row' }, h('button', {
      class: 'btn primary',
      onclick: async () => {
        try {
          const next: Settings = { ...settings, username: user.value.trim(), default_ram_mb: Number(ram.value) || 2048, curseforge_key: cfKey.value.trim() || null };
          await invoke('save_settings', { settings: next });
          settings = next; toast('Saved'); render();
        } catch (e) { toast(errMsg(e), true); }
      },
    }, 'Save')));
}


// ───────────── صفحة Browse: بحث عن مودات / حزم موارد / shaders على Modrinth ─────────────
// اختيار الـ loader إلزامي للمودات (Fabric / Forge / NeoForge / Quilt)؛ التنزيل يذهب إلى instance
// مطابق فقط، فلا تختلط ملفات loader بآخر.
const BROWSE_LOADERS = ['fabric', 'forge', 'neoforge', 'quilt'];
const browseState = { source: 'modrinth', kind: 'mods', loader: '', mc: '', query: '', target: '' };

function renderBrowse(): HTMLElement {
  const st = browseState;
  const wrap = h('div', {}, h('h1', {}, 'Browse'),
    h('p', { class: 'muted' }, 'Search Modrinth and install into one of your instances. Files go only to the instance you pick.'));
  const results = h('div', {}, h('span', { class: 'muted' }, 'Choose what you want and press Search.'));
  const kindBtns = [['mods', 'Mods'], ['resourcepacks', 'Resource packs'], ['shaderpacks', 'Shaders']].map(([k, label]) =>
    h('button', { class: 'btn' + (st.kind === k ? ' primary' : ''), onclick: () => { st.kind = k; st.target = ''; render(); } }, label));
  const loaderSel = h('select', { class: 'input', onchange: (e: Event) => { st.loader = (e.target as HTMLSelectElement).value; st.target = ''; render(); } },
    h('option', { value: '' }, 'Choose a loader…'),
    ...BROWSE_LOADERS.map((l) => h('option', { value: l, selected: st.loader === l }, loaderLabel(l)))) as HTMLSelectElement;
  const mcSel = h('select', { class: 'input', onchange: (e: Event) => { st.mc = (e.target as HTMLSelectElement).value; st.target = ''; render(); } },
    h('option', { value: '' }, 'Any version')) as HTMLSelectElement;
  void ensureVersions().then((ok) => {
    if (!ok) return;
    mcSel.append(...versions.filter((v) => v.kind === 'release').map((v) => h('option', { value: v.id, selected: st.mc === v.id }, v.id)));
  });
  const q = h('input', { class: 'input', placeholder: 'Search…', value: st.query, style: 'width:240px' }) as HTMLInputElement;

  // instances مطابقة: للمودات نفس الـ loader (وإصدار Minecraft إن اختير)؛ للحزم أي instance ماين كرافت.
  const compatible = instances.filter((i) => {
    if (i.loader === 'rust') return false;
    if (st.kind === 'mods' && !(i.loader === st.loader)) return false;
    if (st.mc && i.version_id !== st.mc) return false;
    return true;
  });
  if (!compatible.some((i) => i.id === st.target)) st.target = compatible[0]?.id ?? '';
  const targetSel = h('select', { class: 'input', onchange: (e: Event) => { st.target = (e.target as HTMLSelectElement).value; } },
    ...compatible.map((i) => h('option', { value: i.id, selected: st.target === i.id }, `${i.name} — ${loaderLabel(i.loader)} ${i.version_id}`))) as HTMLSelectElement;

  const needLoader = st.kind === 'mods' && !st.loader;
  const doSearch = async (): Promise<void> => {
    st.query = q.value;
    if (needLoader) { toast('Choose Fabric, Forge, NeoForge or Quilt first', true); return; }
    results.replaceChildren(h('span', { class: 'muted' }, 'Searching…'));
    try {
      const hits: { slug: string; title: string; description: string; downloads: number; icon: string | null; author: string }[] =
        await invoke('browse_search', { source: st.source, kind: st.kind, loader: st.kind === 'mods' ? st.loader : null, mc: st.mc || null, query: q.value });
      results.replaceChildren(...(hits.length ? hits.map((x) => h('div', { class: 'list-item', style: 'padding:6px 0;border-top:1px solid var(--line);display:flex;gap:12px;align-items:center' },
        x.icon ? h('img', { src: x.icon, alt: '', width: '48', height: '48', loading: 'lazy', style: 'border-radius:8px;object-fit:cover;flex:none' })
          : h('div', { style: 'width:48px;height:48px;border-radius:8px;background:var(--line);flex:none' }),
        h('div', { class: 'grow' }, h('strong', {}, x.title), x.author ? h('span', { class: 'muted' }, ` by ${x.author}`) : null, h('div', { class: 'muted' }, x.description),
          h('div', { class: 'muted', style: 'font-size:12px' }, `${x.downloads.toLocaleString()} downloads`)),
        h('button', {
          class: 'btn', disabled: !st.target,
          onclick: async (e: Event) => {
            const b = e.target as HTMLButtonElement; b.disabled = true; b.textContent = 'Installing…';
            try {
              const got: string[] = await invoke('browse_install', { source: st.source, id: st.target, project: x.slug, kind: st.kind });
              toast(got.length ? `Installed ${got.join(', ')}` : 'Already installed');
              b.textContent = 'Installed';
            } catch (err) { toast(errMsg(err), true); b.disabled = false; b.textContent = 'Install'; }
          },
        }, 'Install'))) : [h('span', { class: 'muted' }, 'No results.')]));
    } catch (e) { results.replaceChildren(h('span', { class: 'muted' }, 'Could not search ' + (st.source === 'curseforge' ? 'CurseForge' : 'Modrinth') + ' (' + errMsg(e) + ').')); }
  };
  q.addEventListener('keydown', (e) => { if (e.key === 'Enter') void doSearch(); });

  const noTarget = compatible.length === 0 && !needLoader
    ? h('p', { class: 'muted' }, st.kind === 'mods'
      ? `No ${loaderLabel(st.loader)} instance${st.mc ? ' for ' + st.mc : ''} yet. Create one on the Instances page, then come back to install.`
      : 'No instance yet. Create one on the Instances page first.')
    : null;
  wrap.append(h('div', { class: 'card' },
    h('div', { class: 'row' }, ...[['modrinth', 'Modrinth'], ['curseforge', 'CurseForge']].map(([k, label]) =>
      h('button', { class: 'btn' + (st.source === k ? ' primary' : ''), onclick: () => { st.source = k; render(); } }, label))),
    h('div', { class: 'row', style: 'margin-top:10px' }, ...kindBtns),
    h('div', { class: 'row', style: 'margin-top:10px' }, ...(st.kind === 'mods' ? [loaderSel] : []), mcSel, q,
      h('button', { class: 'btn primary', onclick: () => { void doSearch(); } }, 'Search')),
    h('div', { class: 'row', style: 'margin-top:10px' }, h('span', { class: 'muted' }, 'Install into:'), targetSel),
    noTarget,
    st.kind === 'shaderpacks' ? h('p', { class: 'muted' }, 'Shaders also need a shader mod such as Iris in that instance.') : null,
    st.source === 'curseforge' ? h('p', { class: 'muted', style: 'margin-bottom:0' }, 'CurseForge needs your own API key (Settings). Some authors disable downloads from other apps; those files must be downloaded on curseforge.com. Dependencies are not installed automatically.') : null),
    h('div', { class: 'card' }, results));
  return wrap;
}

// ───────────── التنقل والتشغيل ─────────────
function render(): void {
  consoleEl = null;
  document.querySelectorAll('.nav button').forEach((b) =>
    b.classList.toggle('active', (b as HTMLElement).dataset.page === (page === 'content' ? 'instances' : page)));
  const pages: Record<string, () => HTMLElement> = {
    home: renderHome, instances: renderInstances, browse: renderBrowse, downloads: renderDownloads,
    java: renderJava, console: renderConsole, settings: renderSettings, content: renderContent,
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
