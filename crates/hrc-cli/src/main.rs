//! CLI للاختبار بدون واجهة:
//!   hrc list [release|snapshot|old_beta|old_alpha]
//!   hrc install <version>
//!   hrc launch <version> [username] [ram_mb]     (Offline، للاختبار فقط)
use hrc_core::download::Progress;
use hrc_core::launch::{build_command, spawn, AuthSession, LaunchContext};
use hrc_core::manifest::Manifest;
use hrc_core::platform::{data_dir, Os};
use hrc_core::rules::Features;
use hrc_core::loader::{self, Loader};
use hrc_core::{content, install, java};
use std::sync::{atomic::Ordering, Arc};

#[cfg(windows)]
extern "system" {
    fn SetConsoleOutputCP(code_page: u32) -> i32;
}

#[tokio::main]
async fn main() -> hrc_core::Result<()> {
    #[cfg(windows)]
    unsafe {
        SetConsoleOutputCP(65001); // UTF-8
    }
    let a: Vec<String> = std::env::args().collect();
    if a.get(1).map(String::as_str) == Some("flags") {
        // hrc-cli flags <vanilla|balanced|performance>  → وسائط JVM للـ preset (بدون شبكة)
        let preset = a.get(2).and_then(|p| hrc_core::perf::PerfPreset::parse(p)).unwrap_or_default();
        println!("{}", hrc_core::perf::jvm_flags(preset).join(" "));
        return Ok(());
    }
    let client = reqwest::Client::builder()
        .user_agent("zakadjahra-lgtm/horizon-rust-clean/0.3 (Minecraft launcher CLI)")
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()?;
    if a.get(1).map(String::as_str) == Some("netcheck") {
        // hrc-cli netcheck → يختبر كل الخوادم الرسمية ويعرض سبب أي فشل (للتشخيص)
        let mut bad = 0;
        for c in hrc_core::netcheck::run(&client).await {
            println!("{} {:<24} {}", if c.ok { "OK  " } else { "FAIL" }, c.name, c.detail);
            if !c.ok {
                bad += 1;
            }
        }
        std::process::exit(if bad == 0 { 0 } else { 1 });
    }
    let data = data_dir();
    if a.get(1).map(String::as_str) == Some("mod") {
        // HRC_LOADER=fabric hrc-cli mod <mc-version> <modrinth-slug>  → ينزّل المود + تبعياته إلى mods/ للـ instance
        let l = Loader::parse(&std::env::var("HRC_LOADER").unwrap_or_else(|_| "fabric".into()))
            .ok_or_else(|| hrc_core::Error::Msg("unknown loader".into()))?;
        let (mc, slug) = (a.get(2).expect("missing mc version"), a.get(3).expect("missing mod slug"));
        let game = data.join("instances").join(format!("cli-{}-{mc}", l.label().split_whitespace().next().unwrap_or("x").to_lowercase())).join("game");
        let got = hrc_core::modrinth::install_mod(&client, &game, slug, l, mc).await?;
        println!("Installed mods: {}", got.join(", "));
        for it in hrc_core::content::list_checked(&game, hrc_core::content::Kind::Mods, l, mc)? {
            println!("  {} ({} bytes, enabled={}) {}", it.name, it.size, it.enabled, it.warning.map(|w| format!("WARNING: {w}")).unwrap_or_else(|| "compatible".into()));
        }
        return Ok(());
    }
    let shared = data.join("shared");
    let manifest = Manifest::fetch(&client).await?;

    match a.get(1).map(String::as_str) {
        Some("list") => {
            let kind = a.get(2).map(String::as_str).unwrap_or("release");
            for v in manifest.of_kind(kind) {
                println!("{}", v.id);
            }
        }
        Some("install") | Some("launch") => {
            // نوع البيئة من HRC_LOADER (vanilla|fabric|quilt|forge|neoforge|horizon)، الافتراضي vanilla
            let loader_kind = match std::env::var("HRC_LOADER") {
                Ok(v) => Loader::parse(&v).ok_or_else(|| hrc_core::Error::Msg(format!("unknown loader: {v}")))?,
                Err(_) => Loader::Vanilla,
            };
            let id = a.get(2).expect("missing version id");
            let entry = manifest.find(id).ok_or_else(|| hrc_core::Error::Msg(format!("unknown version: {id}")))?;
            install::install_version_json(&client, &shared, entry).await?;
            let vanilla = install::load_version(&shared, id)?;

            // Vanilla يحتفظ بالمسار القديم (cli-<v>)؛ كل نوع آخر له مجلده المستقل
            let inst = if loader_kind == Loader::Vanilla {
                data.join("instances").join(format!("cli-{id}"))
            } else {
                data.join("instances").join(format!("cli-{}-{id}", loader_kind.label().split_whitespace().next().unwrap_or("x").to_lowercase()))
            };
            let natives = inst.join("natives");
            let progress = Arc::new(Progress::default());

            // Java مطلوب للتشغيل ولتثبيت Forge/NeoForge (installer)
            let java_path = if a[1] == "launch" || loader::needs_vanilla_first(loader_kind) {
                Some(java::ensure_runtime(&client, &data, vanilla.java_component(), progress.clone()).await?)
            } else {
                None
            };
            if loader::needs_vanilla_first(loader_kind) {
                install::install_game_files(&client, &shared, &vanilla, &natives, Os::current(), &Features::default(), progress.clone()).await?;
            }
            let installed = loader::install(&client, &shared, loader_kind, id, java_path.as_deref().unwrap_or(std::path::Path::new(""))).await?;
            let version = install::load_version(&shared, &installed.launch_id)?;
            content::ensure_dirs(&inst.join("game"), loader_kind)?;
            install::install_game_files(&client, &shared, &version, &natives, Os::current(), &Features::default(), progress.clone()).await?;
            println!(
                "Installed {id} [{}] launch id {} ({} files)",
                loader_kind.label(),
                installed.launch_id,
                progress.files_done.load(Ordering::Relaxed)
            );

            if a[1] == "launch" {
                let java_path = java_path.expect("java resolved for launch");
                println!("Java runtime: {}", java_path.display());
                let ctx = LaunchContext {
                    version,
                    shared_dir: shared,
                    game_dir: inst.join("game"),
                    natives_dir: natives,
                    java_path,
                    auth: AuthSession::offline(a.get(3).map(String::as_str).unwrap_or("Player"), "00000000-0000-0000-0000-000000000000"),
                    max_memory_mb: a.get(4).and_then(|s| s.parse().ok()).unwrap_or(2048),
                    min_memory_mb: 512,
                    extra_jvm_args: std::env::var("HRC_EXTRA_JVM")
                        .map(|v| v.split_whitespace().map(String::from).collect())
                        .unwrap_or_default(),
                    features: Features::default(),
                    os: Os::current(),
                };
                let cmd = build_command(&ctx)?;
                let mut child = spawn(&cmd, |is_err, l| if is_err { eprintln!("{l}") } else { println!("{l}") }).await?;
                let status = child.wait().await?;
                println!("Game exited: {status}");
            }
        }
        _ => eprintln!("Usage: [HRC_LOADER=fabric|quilt|forge|neoforge|horizon] hrc-cli list [release|snapshot|old_beta|old_alpha] | install <version> | launch <version> [user] [ram_mb]"),
    }
    Ok(())
}
