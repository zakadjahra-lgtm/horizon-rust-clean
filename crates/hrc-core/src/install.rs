//! تثبيت إصدار: Version JSON + client.jar + libraries + natives + assets (من مصادر Mojang الرسمية).
use crate::download::{fetch, fetch_all, Job, Progress};
use crate::launch::{library_rel_path, native_classifier};
use crate::manifest::ManifestEntry;
use crate::platform::Os;
use crate::rules::{allowed, Features};
use crate::version::{Artifact, VersionJson};
use crate::{err, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
    #[serde(default)]
    map_to_resources: bool,
    #[serde(default, rename = "virtual")]
    is_virtual: bool,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

pub fn version_json_path(shared: &Path, id: &str) -> PathBuf {
    shared.join("versions").join(id).join(format!("{id}.json"))
}

/// يقرأ Version JSON المحلي ويدمج `inheritsFrom` تلقائياً.
pub fn load_version(shared: &Path, id: &str) -> Result<VersionJson> {
    let text = std::fs::read_to_string(version_json_path(shared, id))?;
    let v = VersionJson::parse(&text)?;
    match v.inherits_from.clone() {
        Some(parent) => {
            let p = load_version(shared, &parent)?;
            Ok(v.merged_with_parent(p))
        }
        None => Ok(v),
    }
}

fn art_job(a: &Artifact, dest: PathBuf) -> Job {
    Job { url: a.url.clone(), dest, sha1: a.sha1.clone(), size: a.size }
}

/// ينزّل Version JSON الرسمي (يتحقق من SHA1 المذكور في manifest).
pub async fn install_version_json(
    client: &reqwest::Client,
    shared: &Path,
    entry: &ManifestEntry,
) -> Result<()> {
    let job = Job {
        url: entry.url.clone(),
        dest: version_json_path(shared, &entry.id),
        sha1: entry.sha1.clone(),
        size: None,
    };
    fetch(client, &job, &Progress::default()).await
}

/// يثبّت كل ما يحتاجه الإصدار ضمن `shared/` ويفك natives إلى `natives_dir`.
pub async fn install_game_files(
    client: &reqwest::Client,
    shared: &Path,
    version: &VersionJson,
    natives_dir: &Path,
    os: Os,
    features: &Features,
    progress: Arc<Progress>,
) -> Result<()> {
    let mut jobs: Vec<Job> = Vec::new();
    let mut native_jars: Vec<(PathBuf, Vec<String>)> = Vec::new();

    // 1) client.jar — يُوضع تحت اسم الإصدار الفعلي (للـ inheritsFrom يُنزَّل jar الأب)
    if let Some(c) = version.downloads.as_ref().and_then(|d| d.client.as_ref()) {
        let jar = shared.join("versions").join(&version.id).join(format!("{}.jar", version.id));
        jobs.push(art_job(c, jar));
    }

    // 2) libraries + natives
    for lib in &version.libraries {
        if !allowed(&lib.rules, os, features) {
            continue;
        }
        if let Some(a) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            if let Some(rel) = library_rel_path(lib) {
                jobs.push(art_job(a, shared.join("libraries").join(rel)));
            }
        } else if let (Some(base), Some(rel)) = (&lib.url, crate::launch::maven_path(&lib.name)) {
            // مكتبات Maven بلا downloads (Fabric…): بلا SHA1 رسمي في JSON
            jobs.push(Job {
                url: format!("{}{}", base.trim_end_matches('/').to_owned() + "/", rel),
                dest: shared.join("libraries").join(&rel),
                sha1: None,
                size: None,
            });
        }
        if let Some(cls) = native_classifier(lib, os) {
            if let Some(a) = lib.downloads.as_ref().and_then(|d| d.classifiers.as_ref()).and_then(|c| c.get(&cls)) {
                let rel = a.path.clone().unwrap_or_else(|| format!("natives/{}", cls));
                let dest = shared.join("libraries").join(rel);
                jobs.push(art_job(a, dest.clone()));
                let excl = lib.extract.as_ref().map(|e| e.exclude.clone()).unwrap_or_default();
                native_jars.push((dest, excl));
            }
        }
    }

    // 3) asset index + objects
    if let Some(ai) = &version.asset_index {
        let idx_path = shared.join("assets").join("indexes").join(format!("{}.json", ai.id));
        fetch(
            client,
            &Job { url: ai.url.clone(), dest: idx_path.clone(), sha1: Some(ai.sha1.clone()), size: Some(ai.size) },
            &Progress::default(),
        )
        .await?;
        let idx: AssetIndex = serde_json::from_slice(&tokio::fs::read(&idx_path).await?)?;
        for obj in idx.objects.values() {
            let pre = &obj.hash[..2];
            jobs.push(Job {
                url: format!("{RESOURCES_URL}/{pre}/{}", obj.hash),
                dest: shared.join("assets").join("objects").join(pre).join(&obj.hash),
                sha1: Some(obj.hash.clone()),
                size: Some(obj.size),
            });
        }
        fetch_all(client, jobs, progress, 16).await?;
        materialize_legacy_assets(shared, &ai.id, &idx).await?;
    } else {
        fetch_all(client, jobs, progress, 16).await?;
    }

    // 4) natives → مجلد الـ instance فقط (لا اختلاط بين الإصدارات)
    extract_natives(&native_jars, natives_dir)?;
    Ok(())
}

/// الإصدارات القديمة (virtual / map_to_resources) تحتاج أسماء ملفات حقيقية لا hashes.
async fn materialize_legacy_assets(shared: &Path, id: &str, idx: &AssetIndex) -> Result<()> {
    if !(idx.is_virtual || idx.map_to_resources) {
        return Ok(());
    }
    let target_root = if idx.map_to_resources {
        return Ok(()); // يُنسخ إلى <gameDir>/resources عند التشغيل (انظر instance)
    } else {
        shared.join("assets").join("virtual").join(id)
    };
    for (name, obj) in &idx.objects {
        let src = shared.join("assets").join("objects").join(&obj.hash[..2]).join(&obj.hash);
        let dst = target_root.join(name);
        if dst.exists() {
            continue;
        }
        if let Some(p) = dst.parent() {
            tokio::fs::create_dir_all(p).await?;
        }
        tokio::fs::copy(&src, &dst).await?;
    }
    Ok(())
}

fn extract_natives(jars: &[(PathBuf, Vec<String>)], dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for (jar, exclude) in jars {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(jar)?)?;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i)?;
            let name = f.name().to_string();
            if f.is_dir() || name.starts_with("META-INF") || exclude.iter().any(|e| name.starts_with(e)) {
                continue;
            }
            // حماية من path traversal داخل الأرشيف
            let Some(safe) = f.enclosed_name() else { return err(format!("مسار غير آمن في {name}")) };
            let out = dest.join(safe);
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut o = std::fs::File::create(&out)?;
            std::io::copy(&mut f, &mut o)?;
        }
    }
    Ok(())
}
