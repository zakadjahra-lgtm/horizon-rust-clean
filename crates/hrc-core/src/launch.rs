//! Launch Core: يحوّل Version JSON إلى أمر تشغيل Java صحيح ثم يشغّله.
use crate::platform::{arch_name, Os};
use crate::rules::{allowed, Features};
use crate::version::{Library, VersionJson};
use crate::{err, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    /// "msa" للحسابات المرخصة، "legacy" لوضع Offline.
    pub user_type: String,
}

impl AuthSession {
    pub fn offline(username: &str, uuid: &str) -> Self {
        Self {
            username: username.into(),
            uuid: uuid.into(),
            access_token: "0".into(),
            user_type: "legacy".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LaunchContext {
    pub version: VersionJson,
    /// <data>/shared
    pub shared_dir: PathBuf,
    /// مجلد اللعبة الخاص بالـ instance (saves, options.txt…)
    pub game_dir: PathBuf,
    /// مجلد natives الخاص بالـ instance
    pub natives_dir: PathBuf,
    pub java_path: PathBuf,
    pub auth: AuthSession,
    pub max_memory_mb: u32,
    pub min_memory_mb: u32,
    pub extra_jvm_args: Vec<String>,
    pub features: Features,
    pub os: Os,
}

#[derive(Debug, Clone)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
}

/// مسار Maven النسبي: `g.r.o:artifact:ver[:classifier]` → `g/r/o/artifact/ver/artifact-ver[-classifier].jar`
pub fn maven_path(name: &str) -> Option<String> {
    let (coords, ext) = match name.split_once('@') {
        Some((c, e)) => (c, e),
        None => (name, "jar"),
    };
    let p: Vec<&str> = coords.split(':').collect();
    if p.len() < 3 {
        return None;
    }
    let (g, a, v) = (p[0].replace('.', "/"), p[1], p[2]);
    let cls = p.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    Some(format!("{g}/{a}/{v}/{a}-{v}{cls}.{ext}"))
}

pub fn library_rel_path(lib: &Library) -> Option<String> {
    lib.downloads
        .as_ref()
        .and_then(|d| d.artifact.as_ref())
        .and_then(|a| a.path.clone())
        .or_else(|| maven_path(&lib.name))
}

/// اسم classifier الخاص بالـ natives لهذا النظام (إن وُجد).
pub fn native_classifier(lib: &Library, os: Os) -> Option<String> {
    let c = lib.natives.as_ref()?.get(os.json_name())?;
    let bits = if arch_name() == "x86" { "32" } else { "64" };
    Some(c.replace("${arch}", bits))
}

/// مفتاح إلغاء التكرار: group:artifact[:classifier] (الأول يفوز، والابن قبل الأب).
fn dedupe_key(name: &str) -> String {
    let p: Vec<&str> = name.split(':').collect();
    match p.len() {
        0..=2 => name.to_string(),
        3 => format!("{}:{}", p[0], p[1]),
        _ => format!("{}:{}:{}", p[0], p[1], p[3]),
    }
}

pub fn classpath(ctx: &LaunchContext, client_jar: &Path) -> Result<String> {
    let mut seen = HashSet::new();
    let mut entries: Vec<String> = Vec::new();
    for lib in &ctx.version.libraries {
        if !allowed(&lib.rules, ctx.os, &ctx.features) {
            continue;
        }
        if !seen.insert(dedupe_key(&lib.name)) {
            continue;
        }
        if let Some(rel) = library_rel_path(lib) {
            let p = ctx.shared_dir.join("libraries").join(rel);
            entries.push(p.to_string_lossy().into_owned());
        }
    }
    entries.push(client_jar.to_string_lossy().into_owned());
    Ok(entries.join(ctx.os.classpath_separator()))
}

fn substitute(arg: &str, vars: &[(&str, String)]) -> String {
    let mut s = arg.to_string();
    for (k, v) in vars {
        s = s.replace(&format!("${{{k}}}"), v);
    }
    s
}

/// الإصدارات ≤1.7.2 تستخدم resources، والقديمة جداً `virtual/legacy`.
fn game_assets_dir(ctx: &LaunchContext) -> PathBuf {
    let idx = ctx.version.assets.clone().unwrap_or_else(|| "legacy".into());
    let base = ctx.shared_dir.join("assets");
    if idx == "legacy" || idx == "pre-1.6" {
        base.join("virtual").join(idx)
    } else {
        base
    }
}

pub fn build_command(ctx: &LaunchContext) -> Result<LaunchCommand> {
    let v = &ctx.version;
    let main_class = match &v.main_class {
        Some(m) => m.clone(),
        None => return err("Version JSON لا يحتوي mainClass"),
    };
    let client_jar = ctx
        .shared_dir
        .join("versions")
        .join(&v.id)
        .join(format!("{}.jar", v.id));
    let cp = classpath(ctx, &client_jar)?;

    let assets_index = v.assets.clone().or_else(|| v.asset_index.as_ref().map(|a| a.id.clone()))
        .unwrap_or_else(|| "legacy".into());
    let vars: Vec<(&str, String)> = vec![
        ("auth_player_name", ctx.auth.username.clone()),
        ("auth_uuid", ctx.auth.uuid.clone()),
        ("auth_access_token", ctx.auth.access_token.clone()),
        ("auth_session", format!("token:{}:{}", ctx.auth.access_token, ctx.auth.uuid)),
        ("user_type", ctx.auth.user_type.clone()),
        ("user_properties", "{}".into()),
        ("clientid", String::new()),
        ("auth_xuid", String::new()),
        ("version_name", v.id.clone()),
        ("version_type", v.kind.clone().unwrap_or_else(|| "release".into())),
        ("game_directory", ctx.game_dir.to_string_lossy().into_owned()),
        ("assets_root", ctx.shared_dir.join("assets").to_string_lossy().into_owned()),
        ("assets_index_name", assets_index),
        ("game_assets", game_assets_dir(ctx).to_string_lossy().into_owned()),
        ("natives_directory", ctx.natives_dir.to_string_lossy().into_owned()),
        ("library_directory", ctx.shared_dir.join("libraries").to_string_lossy().into_owned()),
        ("classpath_separator", ctx.os.classpath_separator().into()),
        ("classpath", cp),
        ("launcher_name", "HorizonRustClean".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
    ];

    let (jvm_raw, game_raw) = v.resolved_arguments(ctx.os, &ctx.features);

    let mut args: Vec<String> = vec![
        format!("-Xmx{}M", ctx.max_memory_mb),
        format!("-Xms{}M", ctx.min_memory_mb.min(ctx.max_memory_mb)),
    ];
    args.extend(ctx.extra_jvm_args.iter().cloned());
    args.extend(jvm_raw.iter().map(|a| substitute(a, &vars)));
    args.push(main_class);
    args.extend(game_raw.iter().map(|a| substitute(a, &vars)));

    // حماية: لا يجوز أن يبقى placeholder غير مستبدل في الوسائط.
    if let Some(bad) = args.iter().find(|a| a.contains("${")) {
        return err(format!("placeholder غير معروف في الوسائط: {bad}"));
    }
    Ok(LaunchCommand {
        program: ctx.java_path.clone(),
        args,
        working_dir: ctx.game_dir.clone(),
    })
}

/// يشغّل Minecraft ويمرر كل سطر من stdout/stderr إلى `on_line` (للـ Console في الواجهة).
pub async fn spawn(
    cmd: &LaunchCommand,
    on_line: impl Fn(bool, String) + Send + Sync + Clone + 'static,
) -> Result<Child> {
    tokio::fs::create_dir_all(&cmd.working_dir).await?;
    let mut child = Command::new(&cmd.program)
        .args(&cmd.args)
        .current_dir(&cmd.working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(false)
        .spawn()?;
    if let Some(out) = child.stdout.take() {
        let cb = on_line.clone();
        tokio::spawn(async move {
            let mut l = BufReader::new(out).lines();
            while let Ok(Some(line)) = l.next_line().await {
                cb(false, line);
            }
        });
    }
    if let Some(e) = child.stderr.take() {
        let cb = on_line;
        tokio::spawn(async move {
            let mut l = BufReader::new(e).lines();
            while let Ok(Some(line)) = l.next_line().await {
                cb(true, line);
            }
        });
    }
    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(json: &str, os: Os) -> LaunchContext {
        LaunchContext {
            version: VersionJson::parse(json).unwrap(),
            shared_dir: "/d/shared".into(),
            game_dir: "/d/inst/game".into(),
            natives_dir: "/d/inst/natives".into(),
            java_path: "/j/bin/java".into(),
            auth: AuthSession::offline("Steve", "uuid-1"),
            max_memory_mb: 2048,
            min_memory_mb: 512,
            extra_jvm_args: vec![],
            features: Features::default(),
            os,
        }
    }

    #[test]
    fn maven_paths() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.1:natives-linux").unwrap(),
            "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-linux.jar"
        );
        assert_eq!(maven_path("a.b:c:1").unwrap(), "a/b/c/1/c-1.jar");
    }

    #[test]
    fn legacy_command_has_no_placeholders_left() {
        let c = ctx(
            r#"{"id":"1.8.9","mainClass":"net.minecraft.client.main.Main","assets":"1.8",
                "minecraftArguments":"--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userProperties ${user_properties} --userType ${user_type}",
                "libraries":[{"name":"com.google.guava:guava:17.0","downloads":{"artifact":{"path":"com/google/guava/guava/17.0/guava-17.0.jar","url":"u","sha1":"x","size":1}}}]}"#,
            Os::Linux,
        );
        let cmd = build_command(&c).unwrap();
        let mc = cmd.args.iter().position(|a| a == "net.minecraft.client.main.Main").unwrap();
        // JVM قبل mainClass، وسائط اللعبة بعده
        assert!(cmd.args[..mc].iter().any(|a| a.starts_with("-Djava.library.path=/d/inst/natives")));
        assert!(cmd.args[mc..].contains(&"Steve".to_string()));
        assert!(cmd.args.iter().any(|a| a.contains("guava-17.0.jar") && a.contains("1.8.9.jar")));
    }

    #[test]
    fn modern_command_uses_json_jvm_args() {
        let c = ctx(
            r#"{"id":"1.20.1","mainClass":"net.minecraft.client.main.Main",
                "arguments":{"game":["--username","${auth_player_name}","--version","${version_name}"],
                             "jvm":["-Djava.library.path=${natives_directory}","-cp","${classpath}"]},
                "libraries":[]}"#,
            Os::Linux,
        );
        let cmd = build_command(&c).unwrap();
        assert!(cmd.args.contains(&"-Xmx2048M".to_string()));
        assert!(cmd.args.contains(&"1.20.1".to_string()));
    }

    #[test]
    fn dedupe_prefers_first_library() {
        let c = ctx(
            r#"{"id":"x","mainClass":"M","arguments":{"game":[],"jvm":["-cp","${classpath}"]},
                "libraries":[{"name":"o:a:2"},{"name":"o:a:1"}]}"#,
            Os::Linux,
        );
        let cp = classpath(&c, Path::new("/c.jar")).unwrap();
        assert!(cp.contains("a-2.jar") && !cp.contains("a-1.jar"));
    }
}
