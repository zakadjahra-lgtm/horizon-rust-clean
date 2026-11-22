// لا نافذة طرفية في النسخة النهائية على Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hrc_core::download::Progress;
use hrc_core::instance::{self, Instance};
use hrc_core::launch::{build_command, offline_uuid, spawn, AuthSession, LaunchCommand, LaunchContext};
use hrc_core::loader::{self, Loader};
use hrc_core::manifest::Manifest;
use hrc_core::platform::{data_dir, Os};
use hrc_core::rules::Features;
use hrc_core::profile::{self, Profile};
use hrc_core::settings::{self, Settings};
use hrc_core::{content, install, java, perf};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

struct AppState {
    client: reqwest::Client,
    data: PathBuf,
    manifest: Mutex<Option<Manifest>>,
    /// instances مشغولة (تثبيت/تشغيل) لمنع عمليتين متزامنتين على نفس المجلد.
    busy: Mutex<HashSet<String>>,
    /// تثبيت واحد في كل مرة: instances مختلفة تشترك في ملفات `shared/` ولا يجوز تنزيلها معاً.
    install_lock: Mutex<()>,
}

fn es<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[derive(Serialize, Clone)]
struct VersionInfo {
    id: String,
    kind: String,
}

#[derive(Serialize, Clone)]
struct ProgressEvent {
    phase: &'static str,
    done: u64,
    total: u64,
    files_done: u64,
    files_total: u64,
    speed: f64,
}

#[derive(Serialize, Clone)]
struct ConsoleLine {
    id: String,
    err: bool,
    line: String,
}

#[derive(Serialize, Clone)]
struct IdEvent {
    id: String,
    code: Option<i32>,
    ok: bool,
}

#[derive(Serialize)]
struct JavaInfo {
    installed: Vec<String>,
    manual: Option<String>,
    data_dir: String,
}

async fn with_busy<T>(
    st: &AppState,
    id: &str,
    fut: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    if !st.busy.lock().await.insert(id.to_string()) {
        return Err("This instance is busy (installing or launching)".into());
    }
    let r = fut.await;
    st.busy.lock().await.remove(id);
    r
}

async fn get_manifest(st: &AppState) -> Result<Manifest, String> {
    let mut g = st.manifest.lock().await;
    if g.is_none() {
        *g = Some(Manifest::fetch(&st.client).await.map_err(es)?);
    }
    Ok(g.clone().expect("manifest just set"))
}

/// يرسل حدث `progress` كل 300ms (نسبة + سرعة) حتى يُضبط العلم.
fn start_poller(app: AppHandle, p: Arc<Progress>, phase: &'static str) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let s = stop.clone();
    tauri::async_runtime::spawn(async move {
        let mut last = 0u64;
        let mut last_t = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let done = p.done_bytes.load(Ordering::Relaxed);
            let now = Instant::now();
            let dt = now.duration_since(last_t).as_secs_f64().max(0.001);
            let speed = done.saturating_sub(last) as f64 / dt;
            last = done;
            last_t = now;
            let _ = app.emit(
                "progress",
                ProgressEvent {
                    phase,
                    done,
                    total: p.total_bytes.load(Ordering::Relaxed),
                    files_done: p.files_done.load(Ordering::Relaxed),
                    files_total: p.files_total.load(Ordering::Relaxed),
                    speed,
                },
            );
            if s.load(Ordering::Relaxed) {
                break;
            }
        }
    });
    stop
}

fn manual_java(inst: &Instance, st: &AppState) -> Option<String> {
    let s = settings::load(&st.data);
    inst.java_path
        .clone()
        .filter(|p| !p.trim().is_empty())
        .or_else(|| s.java_path.filter(|p| !p.trim().is_empty()))
}

/// لعبة Rust الأصلية بجانب الـ EXE (يشحنها الـ Installer).
fn rust_game_exe() -> Option<PathBuf> {
    let name = if cfg!(windows) { "horizon-rust-game.exe" } else { "horizon-rust-game" };
    let p = std::env::current_exe().ok()?.parent()?.join(name);
    p.is_file().then_some(p)
}

async fn do_install(app: &AppHandle, st: &AppState, id: &str) -> Result<(), String> {
    let _one_at_a_time = st.install_lock.lock().await;
    let inst = instance::load(&st.data, id).map_err(es)?;
    if inst.loader == Loader::Rust {
        // لعبة أصلية: لا تنزيل من Mojang ولا Java. نتحقق فقط أنها موجودة ونجهّز مجلد الـ instance.
        rust_game_exe().ok_or("horizon-rust-game was not found next to the app (reinstall Horizon Rust Clean)")?;
        std::fs::create_dir_all(instance::game_dir(&st.data, id)).map_err(es)?;
        instance::set_installed(&st.data, id, true).map_err(es)?;
        return Ok(());
    }
    let shared = st.data.join("shared");
    let manifest = get_manifest(st).await?;
    let entry = manifest
        .find(&inst.version_id)
        .ok_or_else(|| format!("Unknown version: {}", inst.version_id))?
        .clone();

    instance::set_installed(&st.data, id, false).map_err(es)?;
    install::install_version_json(&st.client, &shared, &entry).await.map_err(es)?;
    let vanilla = install::load_version(&shared, &inst.version_id).map_err(es)?;
    content::ensure_dirs(&instance::game_dir(&st.data, id), inst.loader).map_err(es)?;

    // natives تُعاد من الصفر لكل تثبيت/إعادة تثبيت (الملفات التالفة تُصلَح عبر SHA1)
    let natives = instance::natives_dir(&st.data, id);
    let _ = tokio::fs::remove_dir_all(&natives).await;

    // Java أولاً: يحتاجه تشغيل installer الخاص بـ Forge/NeoForge أيضاً
    let jp = Arc::new(Progress::default());
    let stop = start_poller(app.clone(), jp.clone(), "java");
    let manual = manual_java(&inst, st);
    let r = java::resolve(&st.client, &st.data, manual.as_deref().map(Path::new), vanilla.java_component(), jp).await;
    stop.store(true, Ordering::Relaxed);
    let java_path = r.map_err(es)?;

    let progress = Arc::new(Progress::default());
    let stop = start_poller(app.clone(), progress.clone(), "game");
    let result: hrc_core::Result<loader::Installed> = async {
        if loader::needs_vanilla_first(inst.loader) {
            install::install_game_files(&st.client, &shared, &vanilla, &natives, Os::current(), &Features::default(), progress.clone()).await?;
        }
        let installed = loader::install(&st.client, &shared, inst.loader, &inst.version_id, &java_path).await?;
        let version = install::load_version(&shared, &installed.launch_id)?;
        install::install_game_files(&st.client, &shared, &version, &natives, Os::current(), &Features::default(), progress.clone()).await?;
        Ok(installed)
    }
    .await;
    stop.store(true, Ordering::Relaxed);
    let installed = result.map_err(es)?;

    instance::set_launch(&st.data, id, installed.loader_version, installed.launch_id).map_err(es)?;
    instance::set_installed(&st.data, id, true).map_err(es)?;
    Ok(())
}

/// ملف الـ Java agent بجانب الـ EXE (يشحنه الـ Installer).
fn agent_jar() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("horizon-client.jar"))
}

/// Profile الخاص بهذا الـ instance (يُنشأ نظيفاً عند أول استخدام ويُربط به).
fn ensure_profile(st: &AppState, inst: &Instance) -> Result<Profile, String> {
    if let Some(pid) = &inst.profile_id {
        if let Ok(p) = profile::load(&st.data, pid) {
            return Ok(p);
        }
    }
    let p = profile::create(&st.data, &inst.name).map_err(es)?;
    instance::set_profile(&st.data, &inst.id, Some(p.id.clone())).map_err(es)?;
    Ok(p)
}

async fn do_launch(app: &AppHandle, st: &AppState, id: &str) -> Result<(), String> {
    let inst = instance::load(&st.data, id).map_err(es)?;
    if !inst.installed {
        return Err("Instance is not installed yet".into());
    }
    let s = settings::load(&st.data);
    if inst.loader == Loader::Rust {
        let exe = rust_game_exe().ok_or("horizon-rust-game was not found next to the app (reinstall Horizon Rust Clean)")?;
        let gd = instance::game_dir(&st.data, id);
        std::fs::create_dir_all(&gd).map_err(es)?;
        let cmd = LaunchCommand {
            program: exe,
            args: vec!["--dir".into(), gd.to_string_lossy().into_owned(), "--user".into(), s.username.clone()],
            working_dir: gd,
        };
        let _ = app.emit("console", ConsoleLine { id: id.to_string(), err: false, line: format!("[launcher] {} [{}] (native Rust game, no Java)", inst.name, inst.loader.label()) });
        return spawn_and_track(app, id, cmd).await;
    }
    let shared = st.data.join("shared");
    let version = install::load_version(&shared, inst.launch_version_id()).map_err(es)?;

    let manual = manual_java(&inst, st);
    let java_path = java::resolve(
        &st.client,
        &st.data,
        manual.as_deref().map(Path::new),
        version.java_component(),
        Arc::new(Progress::default()),
    )
    .await
    .map_err(es)?;

    let sys = |line: String| {
        let _ = app.emit("console", ConsoleLine { id: id.to_string(), err: false, line });
    };

    // Vanilla/Fabric/Quilt/Forge/NeoForge: بلا أي إضافات. RustTools: preset الأداء + Rust mods فقط (بلا agent). Horizon Client: preset الأداء + agent الـ HUD.
    let game_dir = instance::game_dir(&st.data, id);
    let mut extra: Vec<String> = Vec::new();
    if matches!(inst.loader, Loader::Horizon | Loader::RustTools) {
        let prof = ensure_profile(st, &inst)?;
        extra.extend(perf::jvm_flags(prof.perf));
        for line in hrc_core::rustmods::run_prelaunch(&game_dir, &prof.rust_mods, std::time::SystemTime::now()) {
            sys(line);
        }
        if inst.loader == Loader::Horizon && perf::legacy_hud_supported(&inst.version_id) {
            match agent_jar().and_then(|j| profile::agent_arg(&j, &game_dir, &inst.version_id)) {
                Some(arg) => {
                    profile::write_client_config(&game_dir, &prof).map_err(es)?;
                    extra.push(arg);
                }
                None => sys("[launcher] horizon-client.jar not found next to the app — HUD disabled".into()),
            }
        }
    }

    let ctx = LaunchContext {
        version,
        shared_dir: shared,
        game_dir,
        natives_dir: instance::natives_dir(&st.data, id),
        java_path: java_path.clone(),
        auth: AuthSession::offline(&s.username, &offline_uuid(&s.username)),
        max_memory_mb: inst.max_memory_mb,
        min_memory_mb: 512,
        extra_jvm_args: extra,
        features: Features::default(),
        os: Os::current(),
    };
    let cmd = build_command(&ctx).map_err(es)?;

    sys(format!("[launcher] {} [{}] ({}) — java: {}", inst.name, inst.loader.label(), inst.launch_version_id(), java_path.display()));

    spawn_and_track(app, id, cmd).await
}

async fn spawn_and_track(app: &AppHandle, id: &str, cmd: LaunchCommand) -> Result<(), String> {
    let app2 = app.clone();
    let id2 = id.to_string();
    let mut child = spawn(&cmd, move |is_err, line| {
        let _ = app2.emit("console", ConsoleLine { id: id2.clone(), err: is_err, line });
    })
    .await
    .map_err(es)?;

    let _ = app.emit("game-started", IdEvent { id: id.to_string(), code: None, ok: true });
    let (app3, id3) = (app.clone(), id.to_string());
    tauri::async_runtime::spawn(async move {
        let status = child.wait().await;
        let (code, ok) = match status {
            Ok(s) => (s.code(), s.success()),
            Err(_) => (None, false),
        };
        let _ = app3.emit("game-exit", IdEvent { id: id3, code, ok });
    });
    Ok(())
}

fn open_path(p: &Path) -> Result<(), String> {
    std::fs::create_dir_all(p).map_err(es)?;
    #[cfg(target_os = "windows")]
    let prog = "explorer";
    #[cfg(target_os = "macos")]
    let prog = "open";
    #[cfg(all(unix, not(target_os = "macos")))]
    let prog = "xdg-open";
    std::process::Command::new(prog).arg(p).spawn().map_err(es)?;
    Ok(())
}

// ───────────── أوامر الواجهة ─────────────

#[tauri::command]
async fn list_versions(st: State<'_, AppState>) -> Result<Vec<VersionInfo>, String> {
    let m = get_manifest(&st).await?;
    Ok(m.versions.iter().map(|v| VersionInfo { id: v.id.clone(), kind: v.kind.clone() }).collect())
}

#[tauri::command]
async fn list_instances(st: State<'_, AppState>) -> Result<Vec<Instance>, String> {
    instance::list(&st.data).map_err(es)
}

#[tauri::command]
async fn create_instance(
    st: State<'_, AppState>,
    name: String,
    version_id: String,
    max_memory_mb: u32,
    loader: Option<String>,
) -> Result<Instance, String> {
    let kind = match loader.as_deref() {
        None | Some("") => Loader::Vanilla,
        Some(l) => Loader::parse(l).ok_or_else(|| format!("Unknown environment type: {l}"))?,
    };
    instance::create_with(&st.data, &name, &version_id, max_memory_mb, kind).map_err(es)
}

#[tauri::command]
async fn delete_instance(st: State<'_, AppState>, id: String) -> Result<(), String> {
    with_busy(&st, &id, async { instance::delete(&st.data, &id).map_err(es) }).await
}

#[tauri::command]
async fn rename_instance(st: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    instance::rename(&st.data, &id, &name).map_err(es)
}

#[tauri::command]
async fn duplicate_instance(st: State<'_, AppState>, id: String) -> Result<Instance, String> {
    with_busy(&st, &id, async { instance::duplicate(&st.data, &id).map_err(es) }).await
}

#[tauri::command]
async fn set_instance_ram(st: State<'_, AppState>, id: String, mb: u32) -> Result<(), String> {
    instance::set_memory(&st.data, &id, mb).map_err(es)
}

#[tauri::command]
async fn install_instance(app: AppHandle, st: State<'_, AppState>, id: String) -> Result<(), String> {
    let r = with_busy(&st, &id, do_install(&app, &st, &id)).await;
    let _ = app.emit("install-done", IdEvent { id: id.clone(), code: None, ok: r.is_ok() });
    r
}

#[tauri::command]
async fn launch_instance(app: AppHandle, st: State<'_, AppState>, id: String) -> Result<(), String> {
    with_busy(&st, &id, do_launch(&app, &st, &id)).await
}

#[tauri::command]
async fn open_instance_folder(st: State<'_, AppState>, id: String) -> Result<(), String> {
    instance::load(&st.data, &id).map_err(es)?; // يتحقق من صحة المعرّف
    open_path(&instance::game_dir(&st.data, &id))
}

#[tauri::command]
async fn open_data_folder(st: State<'_, AppState>) -> Result<(), String> {
    open_path(&st.data)
}

#[tauri::command]
async fn get_settings(st: State<'_, AppState>) -> Result<Settings, String> {
    Ok(settings::load(&st.data))
}

#[tauri::command]
async fn save_settings(st: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    settings::save(&st.data, &settings).map_err(es)
}

#[tauri::command]
async fn java_info(st: State<'_, AppState>) -> Result<JavaInfo, String> {
    Ok(JavaInfo {
        installed: java::list_installed(&st.data),
        manual: settings::load(&st.data).java_path,
        data_dir: st.data.display().to_string(),
    })
}

// ───────────── محتوى كل instance + إعدادات Horizon Client ─────────────

fn content_kind(k: &str) -> Result<content::Kind, String> {
    content::Kind::parse(k).ok_or_else(|| format!("Unknown content type: {k}"))
}

#[tauri::command]
async fn list_content(st: State<'_, AppState>, id: String, kind: String) -> Result<Vec<content::Item>, String> {
    let inst = instance::load(&st.data, &id).map_err(es)?;
    content::list_checked(&instance::game_dir(&st.data, &id), content_kind(&kind)?, inst.loader, &inst.version_id).map_err(es)
}

#[tauri::command]
async fn toggle_mod(st: State<'_, AppState>, id: String, name: String, enabled: bool) -> Result<(), String> {
    instance::load(&st.data, &id).map_err(es)?;
    content::set_mod_enabled(&instance::game_dir(&st.data, &id), &name, enabled).map_err(es)
}

#[tauri::command]
async fn delete_content(st: State<'_, AppState>, id: String, kind: String, name: String) -> Result<(), String> {
    instance::load(&st.data, &id).map_err(es)?;
    content::delete(&instance::game_dir(&st.data, &id), content_kind(&kind)?, &name).map_err(es)
}

#[tauri::command]
async fn open_content_folder(st: State<'_, AppState>, id: String, kind: String) -> Result<(), String> {
    let inst = instance::load(&st.data, &id).map_err(es)?;
    let game = instance::game_dir(&st.data, &id);
    content::ensure_dirs(&game, inst.loader).map_err(es)?;
    open_path(&content::dir(&game, content_kind(&kind)?))
}

#[tauri::command]
async fn get_client_profile(st: State<'_, AppState>, id: String) -> Result<Profile, String> {
    let inst = instance::load(&st.data, &id).map_err(es)?;
    ensure_profile(&st, &inst)
}

#[tauri::command]
async fn save_client_profile(st: State<'_, AppState>, id: String, profile: Profile) -> Result<Profile, String> {
    let inst = instance::load(&st.data, &id).map_err(es)?;
    let current = ensure_profile(&st, &inst)?;
    // لا نسمح للواجهة بكتابة profile آخر: المعرّف يُفرَض من الـ instance
    let mut p = profile;
    p.id = current.id;
    profile::save(&st.data, &p).map_err(es)
}

#[derive(serde::Serialize)]
struct Curated { slug: String, note: String }

#[tauri::command]
async fn curated_mods() -> Vec<Curated> {
    hrc_core::modrinth::CURATED_FABRIC.iter().map(|(s, n)| Curated { slug: s.to_string(), note: n.to_string() }).collect()
}

#[tauri::command]
async fn search_mods(st: State<'_, AppState>, id: String, query: String) -> Result<Vec<hrc_core::modrinth::Hit>, String> {
    let inst = instance::load(&st.data, &id).map_err(es)?;
    hrc_core::modrinth::search(&st.client, &query, inst.loader, &inst.version_id).await.map_err(es)
}

#[tauri::command]
async fn install_mod(st: State<'_, AppState>, id: String, project: String) -> Result<Vec<String>, String> {
    let inst = instance::load(&st.data, &id).map_err(es)?;
    hrc_core::modrinth::install_mod(&st.client, &instance::game_dir(&st.data, &id), &project, inst.loader, &inst.version_id)
        .await
        .map_err(es)
}

#[tauri::command]
async fn net_check(st: State<'_, AppState>) -> Result<Vec<hrc_core::netcheck::Check>, String> {
    Ok(hrc_core::netcheck::run(&st.client).await)
}

fn main() {
    let client = reqwest::Client::builder()
        .user_agent(concat!("zakadjahra-lgtm/horizon-rust-clean/", env!("CARGO_PKG_VERSION"), " (Minecraft launcher)"))
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .expect("http client");
    let state = AppState {
        client,
        data: data_dir(),
        manifest: Mutex::new(None),
        busy: Mutex::new(HashSet::new()),
        install_lock: Mutex::new(()),
    };
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_versions,
            curated_mods,
            net_check,
            search_mods,
            install_mod,
            list_instances,
            create_instance,
            delete_instance,
            rename_instance,
            duplicate_instance,
            set_instance_ram,
            install_instance,
            launch_instance,
            open_instance_folder,
            open_data_folder,
            get_settings,
            save_settings,
            java_info,
            list_content,
            toggle_mod,
            delete_content,
            open_content_folder,
            get_client_profile,
            save_client_profile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Horizon Rust Clean");
}
