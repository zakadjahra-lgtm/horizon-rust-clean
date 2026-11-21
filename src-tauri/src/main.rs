// لا نافذة طرفية في النسخة النهائية على Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hrc_core::download::Progress;
use hrc_core::instance::{self, Instance};
use hrc_core::launch::{build_command, offline_uuid, spawn, AuthSession, LaunchContext};
use hrc_core::manifest::Manifest;
use hrc_core::platform::{data_dir, Os};
use hrc_core::rules::Features;
use hrc_core::settings::{self, Settings};
use hrc_core::{install, java};
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

async fn do_install(app: &AppHandle, st: &AppState, id: &str) -> Result<(), String> {
    let inst = instance::load(&st.data, id).map_err(es)?;
    let shared = st.data.join("shared");
    let manifest = get_manifest(st).await?;
    let entry = manifest
        .find(&inst.version_id)
        .ok_or_else(|| format!("Unknown version: {}", inst.version_id))?
        .clone();

    instance::set_installed(&st.data, id, false).map_err(es)?;
    install::install_version_json(&st.client, &shared, &entry).await.map_err(es)?;
    let version = install::load_version(&shared, &inst.version_id).map_err(es)?;

    // natives تُعاد من الصفر لكل تثبيت/إعادة تثبيت (الملفات التالفة تُصلَح عبر SHA1)
    let natives = instance::natives_dir(&st.data, id);
    let _ = tokio::fs::remove_dir_all(&natives).await;

    let progress = Arc::new(Progress::default());
    let stop = start_poller(app.clone(), progress.clone(), "game");
    let r = install::install_game_files(
        &st.client,
        &shared,
        &version,
        &natives,
        Os::current(),
        &Features::default(),
        progress.clone(),
    )
    .await;
    stop.store(true, Ordering::Relaxed);
    r.map_err(es)?;

    // Java المناسب يُحمَّل الآن حتى يكون Play سريعاً لاحقاً
    let jp = Arc::new(Progress::default());
    let stop = start_poller(app.clone(), jp.clone(), "java");
    let manual = manual_java(&inst, st);
    let r = java::resolve(
        &st.client,
        &st.data,
        manual.as_deref().map(Path::new),
        version.java_component(),
        jp,
    )
    .await;
    stop.store(true, Ordering::Relaxed);
    r.map_err(es)?;

    instance::set_installed(&st.data, id, true).map_err(es)?;
    Ok(())
}

async fn do_launch(app: &AppHandle, st: &AppState, id: &str) -> Result<(), String> {
    let inst = instance::load(&st.data, id).map_err(es)?;
    if !inst.installed {
        return Err("Instance is not installed yet".into());
    }
    let s = settings::load(&st.data);
    let shared = st.data.join("shared");
    let version = install::load_version(&shared, &inst.version_id).map_err(es)?;

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

    let ctx = LaunchContext {
        version,
        shared_dir: shared,
        game_dir: instance::game_dir(&st.data, id),
        natives_dir: instance::natives_dir(&st.data, id),
        java_path: java_path.clone(),
        auth: AuthSession::offline(&s.username, &offline_uuid(&s.username)),
        max_memory_mb: inst.max_memory_mb,
        min_memory_mb: 512,
        extra_jvm_args: vec![],
        features: Features::default(),
        os: Os::current(),
    };
    let cmd = build_command(&ctx).map_err(es)?;

    let sys = |line: String| {
        let _ = app.emit("console", ConsoleLine { id: id.to_string(), err: false, line });
    };
    sys(format!("[launcher] {} ({}) — java: {}", inst.name, inst.version_id, java_path.display()));

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
) -> Result<Instance, String> {
    instance::create(&st.data, &name, &version_id, max_memory_mb).map_err(es)
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

fn main() {
    let client = reqwest::Client::builder()
        .user_agent(concat!("HorizonRustClean/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("http client");
    let state = AppState {
        client,
        data: data_dir(),
        manifest: Mutex::new(None),
        busy: Mutex::new(HashSet::new()),
    };
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_versions,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Horizon Rust Clean");
}
