//! Java Runtime Manager: يستخدم Runtimes الرسمية من Mojang ويحتفظ بعدة إصدارات في `runtimes/`.
use crate::download::{fetch_all, Job, Progress};
use crate::platform::{java_platform_key, Os};
use crate::{err, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const ALL_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Deserialize)]
struct Entry {
    manifest: Ref,
}

#[derive(Debug, Deserialize)]
struct Ref {
    sha1: String,
    size: u64,
    url: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RFile>,
}

#[derive(Debug, Deserialize)]
struct RFile {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    executable: bool,
    #[serde(default)]
    downloads: Option<RDownloads>,
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RDownloads {
    raw: Ref,
}

pub fn runtime_dir(data: &Path, component: &str) -> PathBuf {
    data.join("runtimes").join(component)
}

/// مسار ملف java التنفيذي داخل مجلد Runtime (بحسب النظام).
pub fn java_executable(root: &Path) -> PathBuf {
    match Os::current() {
        Os::Windows => root.join("bin").join("javaw.exe"),
        Os::Osx => root.join("jre.bundle/Contents/Home/bin/java"),
        Os::Linux => root.join("bin").join("java"),
    }
}

/// هل Runtime مثبّت وجاهز؟ (علامة `.complete` تُكتب بعد نجاح كل الملفات)
pub fn is_installed(data: &Path, component: &str) -> bool {
    let root = runtime_dir(data, component);
    root.join(".complete").exists() && java_executable(&root).exists()
}

/// يعيد مسار java لهذا المكوّن، ويحمّله من Mojang إن لم يكن موجوداً.
pub async fn ensure_runtime(
    client: &reqwest::Client,
    data: &Path,
    component: &str,
    progress: Arc<Progress>,
) -> Result<PathBuf> {
    let root = runtime_dir(data, component);
    if is_installed(data, component) {
        return Ok(java_executable(&root));
    }

    let all: HashMap<String, HashMap<String, Vec<Entry>>> =
        client.get(ALL_URL).send().await?.error_for_status()?.json().await?;
    let platform = java_platform_key();
    let entry = all
        .get(platform)
        .and_then(|p| p.get(component))
        .and_then(|v| v.first());
    let entry = match entry {
        Some(e) => e,
        None => {
            return err(format!(
                "لا يوجد Runtime رسمي '{component}' لمنصة {platform}. حدّد Java يدوياً من الإعدادات."
            ))
        }
    };

    let man: RuntimeManifest = client
        .get(&entry.manifest.url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut jobs = Vec::new();
    let mut exec = Vec::new();
    let mut links = Vec::new();
    for (rel, f) in &man.files {
        let dest = root.join(rel);
        match f.kind.as_str() {
            "directory" => tokio::fs::create_dir_all(&dest).await?,
            "file" => {
                if let Some(d) = &f.downloads {
                    jobs.push(Job {
                        url: d.raw.url.clone(),
                        dest: dest.clone(),
                        sha1: Some(d.raw.sha1.clone()),
                        size: Some(d.raw.size),
                    });
                    if f.executable {
                        exec.push(dest);
                    }
                }
            }
            "link" => {
                if let Some(t) = &f.target {
                    links.push((dest, t.clone()));
                }
            }
            _ => {}
        }
    }
    fetch_all(client, jobs, progress, 16).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for p in &exec {
            let mut perm = std::fs::metadata(p)?.permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(p, perm)?;
        }
        for (link, target) in &links {
            if let Some(parent) = link.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let _ = std::fs::remove_file(link);
            std::os::unix::fs::symlink(target, link)?;
        }
    }
    #[cfg(not(unix))]
    let _ = (&exec, &links);

    tokio::fs::write(root.join(".complete"), entry.manifest.sha1.as_bytes()).await?;
    Ok(java_executable(&root))
}

/// يختار Java: المسار اليدوي إن وُجد، وإلا Runtime الرسمي المناسب للإصدار.
pub async fn resolve(
    client: &reqwest::Client,
    data: &Path,
    manual: Option<&Path>,
    component: &str,
    progress: Arc<Progress>,
) -> Result<PathBuf> {
    if let Some(p) = manual {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        return err(format!("مسار Java اليدوي غير موجود: {}", p.display()));
    }
    ensure_runtime(client, data, component, progress).await
}

/// Runtimes المثبّتة (لصفحة Java Management).
pub fn list_installed(data: &Path) -> Vec<String> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(data.join("runtimes")) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if is_installed(data, &name) {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_json_shape() {
        let j = r#"{"linux":{"java-runtime-gamma":[{"manifest":{"sha1":"s","size":1,"url":"u"},"version":{"name":"17"}}]}}"#;
        let all: HashMap<String, HashMap<String, Vec<Entry>>> = serde_json::from_str(j).unwrap();
        assert_eq!(all["linux"]["java-runtime-gamma"][0].manifest.url, "u");
    }

    #[test]
    fn parses_runtime_manifest() {
        let j = r#"{"files":{"bin":{"type":"directory"},
          "bin/java":{"type":"file","executable":true,"downloads":{"raw":{"sha1":"s","size":2,"url":"u"}}},
          "lib/x":{"type":"link","target":"../y"}}}"#;
        let m: RuntimeManifest = serde_json::from_str(j).unwrap();
        assert!(m.files["bin/java"].executable);
        assert_eq!(m.files["lib/x"].target.as_deref(), Some("../y"));
    }
}
