//! CurseForge (API oficial: https://docs.curseforge.com). يحتاج مفتاح API يضعه المستخدم في الإعدادات
//! (`Settings::curseforge_key`) أو في متغير البيئة `HRC_CURSEFORGE_KEY`؛ لا يُضمَّن أي مفتاح في الكود.
//! بعض المؤلفين يعطّلون التنزيل من تطبيقات خارجية: عندها يعيد الـ API رابطاً فارغاً ونقول ذلك بوضوح.
use crate::content::{self, safe_file_name, Kind};
use crate::download::{fetch, Job, Progress};
use crate::loader::Loader;
use crate::modrinth::Hit;
use crate::{err, Result};
use serde::Deserialize;
use std::path::Path;

const API: &str = "https://api.curseforge.com/v1";
/// معرّف لعبة Minecraft على CurseForge.
const GAME_ID: u32 = 432;

pub fn class_id(kind: Kind) -> u32 {
    match kind {
        Kind::Mods => 6,
        Kind::ResourcePacks => 12,
        Kind::ShaderPacks => 6552,
    }
}

/// `modLoaderType` في CurseForge.
pub fn loader_type(l: Loader) -> Option<u32> {
    match l {
        Loader::Forge => Some(1),
        Loader::Fabric => Some(4),
        Loader::Quilt => Some(5),
        Loader::Neoforge => Some(6),
        _ => None,
    }
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

pub fn search_url(query: &str, kind: Kind, loader: Option<Loader>, mc: Option<&str>) -> Option<String> {
    let mut u = format!("{API}/mods/search?gameId={GAME_ID}&classId={}&pageSize=20&sortField=2&sortOrder=desc&searchFilter={}", class_id(kind), enc(query));
    if kind == Kind::Mods {
        u.push_str(&format!("&modLoaderType={}", loader_type(loader?)?));
    }
    if let Some(v) = mc.filter(|v| !v.is_empty()) {
        u.push_str(&format!("&gameVersion={}", enc(v)));
    }
    Some(u)
}

#[derive(Deserialize)]
struct SearchResp {
    data: Vec<Mod>,
}
#[derive(Deserialize)]
struct Mod {
    id: u64,
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default, rename = "downloadCount")]
    download_count: f64,
    #[serde(default)]
    logo: Option<Logo>,
    #[serde(default)]
    authors: Vec<Author>,
}
#[derive(Deserialize)]
struct Logo {
    #[serde(default, rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
}
#[derive(Deserialize)]
struct Author {
    name: String,
}
#[derive(Deserialize)]
struct FilesResp {
    data: Vec<FileInfo>,
}
#[derive(Deserialize)]
struct FileInfo {
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(default, rename = "downloadUrl")]
    download_url: Option<String>,
    #[serde(default, rename = "releaseType")]
    release_type: u32,
    #[serde(default, rename = "fileDate")]
    file_date: String,
    #[serde(default, rename = "fileLength")]
    file_length: u64,
    #[serde(default)]
    hashes: Vec<HashInfo>,
}
#[derive(Deserialize)]
struct HashInfo {
    value: String,
    algo: u32,
}

/// `slug` في النتيجة هو معرّف المشروع الرقمي (يحتاجه التنزيل).
pub fn parse_search(body: &str) -> Result<Vec<Hit>> {
    let r: SearchResp = serde_json::from_str(body)?;
    Ok(r.data.into_iter().map(|m| Hit {
        slug: m.id.to_string(),
        title: m.name,
        description: m.summary,
        downloads: m.download_count as u64,
        icon: crate::modrinth::safe_icon(m.logo.and_then(|l| l.thumbnail_url)),
        author: m.authors.into_iter().next().map(|a| a.name).unwrap_or_default(),
    }).collect())
}

/// الأحدث من نوع release (releaseType 1)، وإلا الأحدث أياً كان.
fn pick(files: &mut [FileInfo]) -> Option<&FileInfo> {
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
    let i = files.iter().position(|f| f.release_type == 1).or(if files.is_empty() { None } else { Some(0) })?;
    files.get(i)
}

/// التنزيل يجب أن يكون من CDN الخاص بـ CurseForge فقط.
pub fn trusted_url(u: &str) -> bool {
    u.strip_prefix("https://").and_then(|r| r.split('/').next()).is_some_and(|h| h == "forgecdn.net" || h.ends_with(".forgecdn.net"))
}

fn key_or_err(key: Option<&str>) -> Result<&str> {
    match key.map(str::trim).filter(|k| !k.is_empty()) {
        Some(k) => Ok(k),
        None => err("CurseForge needs an API key: add yours in Settings (or set HRC_CURSEFORGE_KEY)"),
    }
}

pub async fn search(client: &reqwest::Client, key: Option<&str>, query: &str, kind: Kind, loader: Option<Loader>, mc: Option<&str>) -> Result<Vec<Hit>> {
    let key = key_or_err(key)?;
    let Some(url) = search_url(query, kind, loader, mc) else { return err("اختر Fabric أو Forge أو NeoForge أو Quilt للبحث عن مودات") };
    let body = client.get(url).header("x-api-key", key).send().await?.error_for_status()?.text().await?;
    parse_search(&body)
}

/// يثبّت ملف المشروع (بدون تبعيات) في مجلد النوع داخل الـ instance.
pub async fn install(client: &reqwest::Client, key: Option<&str>, game_dir: &Path, project: &str, kind: Kind, loader: Loader, mc: &str) -> Result<Vec<String>> {
    let key = key_or_err(key)?;
    if !project.chars().all(|c| c.is_ascii_digit()) || project.is_empty() {
        return err("معرّف مشروع غير صالح");
    }
    let mut url = format!("{API}/mods/{project}/files?gameVersion={}&pageSize=50", enc(mc));
    if kind == Kind::Mods {
        let Some(t) = loader_type(loader) else { return err("هذه البيئة لا تدعم المودات") };
        url.push_str(&format!("&modLoaderType={t}"));
    }
    let body = client.get(url).header("x-api-key", key).send().await?.error_for_status()?.text().await?;
    let mut r: FilesResp = serde_json::from_str(&body)?;
    let Some(f) = pick(&mut r.data) else { return err(format!("لا يوجد ملف لـ {mc} على CurseForge")) };
    let Some(dl) = f.download_url.as_deref() else {
        return err("The author disabled downloads from other apps for this file. Open it on curseforge.com and download it by hand.");
    };
    let ext_ok = if kind == Kind::Mods { f.file_name.ends_with(".jar") } else { f.file_name.ends_with(".zip") };
    if !trusted_url(dl) || !safe_file_name(&f.file_name) || !ext_ok {
        return err("رابط أو اسم ملف غير آمن من المصدر");
    }
    let folder = content::dir(game_dir, kind);
    std::fs::create_dir_all(&folder)?;
    let dest = folder.join(&f.file_name);
    if dest.exists() || folder.join(format!("{}.disabled", f.file_name)).exists() {
        return Ok(Vec::new());
    }
    let sha1 = f.hashes.iter().find(|h| h.algo == 1).map(|h| h.value.clone());
    let job = Job { url: dl.to_string(), dest, sha1, size: if f.file_length > 0 { Some(f.file_length) } else { None } };
    fetch(client, &job, &Progress::default()).await?;
    Ok(vec![f.file_name.clone()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_url_has_class_loader_and_version() {
        let u = search_url("sodium extra", Kind::Mods, Some(Loader::Fabric), Some("1.20.1")).unwrap();
        assert!(u.contains("gameId=432") && u.contains("classId=6") && u.contains("modLoaderType=4"));
        assert!(u.contains("searchFilter=sodium%20extra") && u.contains("gameVersion=1.20.1"));
        assert!(search_url("x", Kind::Mods, None, None).is_none());
        let p = search_url("x", Kind::ShaderPacks, None, None).unwrap();
        assert!(p.contains("classId=6552") && !p.contains("modLoaderType"));
        assert_eq!(loader_type(Loader::Neoforge), Some(6));
        assert_eq!(loader_type(Loader::Vanilla), None);
    }

    #[test]
    fn parses_a_search_reply() {
        let body = r#"{"data":[{"id":394468,"name":"Sodium","summary":"Fast","downloadCount":1.5e7,"slug":"sodium","logo":{"thumbnailUrl":"https://media.forgecdn.net/s.png"},"authors":[{"name":"jellysquid3"}]}],"pagination":{}}"#;
        let hits = parse_search(body).unwrap();
        assert_eq!((hits[0].slug.as_str(), hits[0].title.as_str(), hits[0].downloads), ("394468", "Sodium", 15_000_000));
        assert_eq!((hits[0].icon.as_deref(), hits[0].author.as_str()), (Some("https://media.forgecdn.net/s.png"), "jellysquid3"));
    }

    #[test]
    fn picks_the_newest_release_and_trusts_only_the_cdn() {
        let body = r#"{"data":[
          {"fileName":"b.jar","downloadUrl":null,"releaseType":2,"fileDate":"2026-03-01T00:00:00Z"},
          {"fileName":"old.jar","downloadUrl":"https://edge.forgecdn.net/o.jar","releaseType":1,"fileDate":"2026-01-01T00:00:00Z"},
          {"fileName":"new.jar","downloadUrl":"https://mediafilez.forgecdn.net/n.jar","releaseType":1,"fileDate":"2026-02-01T00:00:00Z","hashes":[{"value":"ab","algo":1}]}]}"#;
        let mut r: FilesResp = serde_json::from_str(body).unwrap();
        assert_eq!(pick(&mut r.data).unwrap().file_name, "new.jar");
        assert!(trusted_url("https://edge.forgecdn.net/x.jar") && trusted_url("https://forgecdn.net/x"));
        assert!(!trusted_url("http://edge.forgecdn.net/x") && !trusted_url("https://evil-forgecdn.net/x") && !trusted_url("https://forgecdn.net.evil.com/x"));
    }

    #[test]
    fn no_key_is_a_clear_error() {
        assert!(key_or_err(None).is_err() && key_or_err(Some("  ")).is_err());
        assert_eq!(key_or_err(Some(" k ")).unwrap(), "k");
    }
}
