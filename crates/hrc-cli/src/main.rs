//! CLI للاختبار بدون واجهة:
//!   hrc list [release|snapshot|old_beta|old_alpha]
//!   hrc install <version>
//!   hrc launch <version> [username] [ram_mb]     (Offline، للاختبار فقط)
use hrc_core::download::Progress;
use hrc_core::launch::{build_command, spawn, AuthSession, LaunchContext};
use hrc_core::manifest::Manifest;
use hrc_core::platform::{data_dir, Os};
use hrc_core::rules::Features;
use hrc_core::{install, java};
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
    let client = reqwest::Client::builder().user_agent("HorizonRustClean/0.1").build()?;
    let data = data_dir();
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
            let id = a.get(2).expect("missing version id");
            let entry = manifest.find(id).ok_or_else(|| hrc_core::Error::Msg(format!("unknown version: {id}")))?;
            install::install_version_json(&client, &shared, entry).await?;
            let version = install::load_version(&shared, id)?;

            let inst = data.join("instances").join(format!("cli-{id}"));
            let natives = inst.join("natives");
            let progress = Arc::new(Progress::default());
            install::install_game_files(&client, &shared, &version, &natives, Os::current(), &Features::default(), progress.clone()).await?;
            println!("Installed {id} ({} files)", progress.files_done.load(Ordering::Relaxed));

            if a[1] == "launch" {
                let java_path = java::ensure_runtime(&client, &data, version.java_component(), progress).await?;
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
        _ => eprintln!("Usage: hrc-cli list [release|snapshot|old_beta|old_alpha] | install <version> | launch <version> [user] [ram_mb]"),
    }
    Ok(())
}
