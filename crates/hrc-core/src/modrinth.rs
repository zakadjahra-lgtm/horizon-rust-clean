//! مدير مودات يعتمد API الرسمي لـ Modrinth (https://docs.modrinth.com). لا نضمّن أي مود في اللانشر؛
//! يُنزَّل من Modrinth عند طلب المستخدم، مع التحقق من SHA1 الذي يعطيه Modrinth، داخل مجلد mods لذلك الـ instance فقط.
use crate::content::{self, safe_file_name, Kind};
use crate::download::{fetch, Job, Progress};
use crate::loader::Loader;
use crate::{err, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

const API: &str = "https://api.modrinth.com/v2";

/// مودات Fabric خفيفة معروفة (slugs على Modrinth). قائمة اقتراح فقط؛ التوافق مع الإصدار يُتحقَّق منه عند التحميل.
pub const CURATED_FABRIC: &[(&str, &str)] = &[
    ("sodium", "Rendering engine: big FPS gains (PolyForm Shield, downloaded from Modrinth, not bundled)"),
    ("lithium", "Game logic / tick optimizations"),
    ("ferrite-core", "Lower memory usage"),
    ("entityculling", "Skips rendering hidden entities"),
    ("immediatelyfast", "Faster immediate-mode rendering (HUD, GUI)"),
    ("modernfix", "Startup time and memory fixes"),
    ("dynamic-fps", "Lowers FPS when the window is in the background"),
    ("modmenu", "In-game list of installed mods"),
    ("iris", "Shader support (needs Sodium)"),
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Hit {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
}

#[derive(Debug, Deserialize)]
struct SearchResp {
    hits: Vec<RawHit>,
}
#[derive(Debug, Deserialize)]
struct RawHit {
    slug: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    downloads: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    #[serde(default)]
    pub version_type: String,
    #[serde(default)]
    pub version_number: String,
    pub files: Vec<File>,
    #[serde(default)]
    pub dependencies: Vec<Dep>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct File {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: u64,
    pub hashes: Hashes,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Hashes {
    #[serde(default)]
    pub sha1: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Dep {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub dependency_type: String,
}

/// ترميز بسيط للـ query (RFC 3986 unreserved فقط).
fn enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

pub fn search_url(query: &str, loader: Loader, mc: &str) -> Option<String> {
    let l = loader.modrinth_loader()?;
    let facets = format!(r#"[["project_type:mod"],["categories:{l}"],["versions:{mc}"]]"#);
    Some(format!("{API}/search?query={}&facets={}&limit=20&index=downloads", enc(query), enc(&facets)))
}

pub fn versions_url(project: &str, loader: Loader, mc: &str) -> Option<String> {
    let l = loader.modrinth_loader()?;
    Some(format!(
        "{API}/project/{}/version?loaders={}&game_versions={}",
        enc(project),
        enc(&format!(r#"["{l}"]"#)),
        enc(&format!(r#"["{mc}"]"#))
    ))
}

/// الأحدث من نوع release، وإلا الأحدث أياً كان (Modrinth يرتّب الأحدث أولاً).
pub fn pick_version(vs: &[Version]) -> Option<&Version> {
    vs.iter().find(|v| v.version_type == "release").or_else(|| vs.first())
}

pub fn pick_file(v: &Version) -> Option<&File> {
    v.files.iter().find(|f| f.primary).or_else(|| v.files.first())
}

pub async fn search(client: &reqwest::Client, query: &str, loader: Loader, mc: &str) -> Result<Vec<Hit>> {
    let Some(loader) = loader.mods_loader(mc) else { return err("هذه البيئة لا تدعم المودات") };
    let Some(url) = search_url(query, loader, mc) else { return err("هذه البيئة لا تدعم المودات") };
    let r: SearchResp = client.get(url).send().await?.error_for_status()?.json().await?;
    Ok(r.hits.into_iter().map(|h| Hit { slug: h.slug, title: h.title, description: h.description, downloads: h.downloads }).collect())
}

async fn versions(client: &reqwest::Client, project: &str, loader: Loader, mc: &str) -> Result<Vec<Version>> {
    let Some(url) = versions_url(project, loader, mc) else { return err("هذه البيئة لا تدعم المودات") };
    Ok(client.get(url).send().await?.error_for_status()?.json().await?)
}

/// يثبّت مودّاً (project id أو slug) مع تبعياته الإلزامية (مستوى واحد) داخل mods/ للـ instance.
/// يعيد أسماء الملفات التي نُزّلت فعلاً.
pub async fn install_mod(
    client: &reqwest::Client,
    game_dir: &Path,
    project: &str,
    loader: Loader,
    mc: &str,
) -> Result<Vec<String>> {
    let Some(loader) = loader.mods_loader(mc) else { return err("هذه البيئة لا تدعم المودات") };
    content::ensure_dirs(game_dir, loader)?;
    let mut queue = vec![project.to_string()];
    let mut seen: Vec<String> = Vec::new();
    let mut done = Vec::new();
    let mut depth_one = true;
    while let Some(p) = queue.pop() {
        if seen.contains(&p) {
            continue;
        }
        seen.push(p.clone());
        let vs = versions(client, &p, loader, mc).await?;
        let Some(v) = pick_version(&vs) else {
            if depth_one {
                return err(format!("لا يوجد إصدار من هذا المود لـ {mc} على {}", loader.label()));
            }
            continue; // تبعية بلا إصدار مناسب: نتجاوزها ولا نفشل
        };
        let Some(f) = pick_file(v) else { continue };
        if !safe_file_name(&f.filename) || !f.filename.ends_with(".jar") {
            return err("اسم ملف غير آمن من المصدر");
        }
        let dest = content::dir(game_dir, Kind::Mods).join(&f.filename);
        let off = content::dir(game_dir, Kind::Mods).join(format!("{}.disabled", f.filename));
        if !dest.exists() && !off.exists() {
            let job = Job { url: f.url.clone(), dest, sha1: f.hashes.sha1.clone(), size: if f.size > 0 { Some(f.size) } else { None } };
            fetch(client, &job, &Progress::default()).await?;
            done.push(f.filename.clone());
        }
        if depth_one {
            for d in &v.dependencies {
                if d.dependency_type == "required" {
                    if let Some(id) = &d.project_id {
                        queue.push(id.clone());
                    }
                }
            }
            depth_one = false;
        }
    }
    Ok(done)
}

/// ما يبحث عنه صفحة Browse: مودات (لها loader إلزامي)، حزم موارد، أو shaders.
fn project_type(kind: Kind) -> &'static str {
    match kind {
        Kind::Mods => "mod",
        Kind::ResourcePacks => "resourcepack",
        Kind::ShaderPacks => "shader",
    }
}

/// رابط بحث عام. للمودات يلزم loader حقيقي (Fabric/Quilt/Forge/NeoForge) وإلا `None`؛ حزم الموارد
/// والـ shaders لا ترتبط بـ loader. إصدار Minecraft اختياري.
pub fn browse_url(query: &str, kind: Kind, loader: Option<Loader>, mc: Option<&str>) -> Option<String> {
    let mut facets = vec![vec![format!("project_type:{}", project_type(kind))]];
    if kind == Kind::Mods {
        facets.push(vec![format!("categories:{}", loader?.modrinth_loader()?)]);
    }
    if let Some(v) = mc.filter(|v| !v.is_empty()) {
        facets.push(vec![format!("versions:{v}")]);
    }
    let facets = serde_json::to_string(&facets).ok()?;
    Some(format!("{API}/search?query={}&facets={}&limit=20&index=relevance", enc(query), enc(&facets)))
}

pub async fn browse(client: &reqwest::Client, query: &str, kind: Kind, loader: Option<Loader>, mc: Option<&str>) -> Result<Vec<Hit>> {
    let Some(url) = browse_url(query, kind, loader, mc) else { return err("اختر Fabric أو Forge أو NeoForge أو Quilt للبحث عن مودات") };
    let r: SearchResp = client.get(url).send().await?.error_for_status()?.json().await?;
    Ok(r.hits.into_iter().map(|h| Hit { slug: h.slug, title: h.title, description: h.description, downloads: h.downloads }).collect())
}

/// ينزّل حزمة موارد أو shader (ملف .zip واحد) إلى مجلدها داخل الـ instance، مع SHA1 من Modrinth.
pub async fn install_pack(client: &reqwest::Client, game_dir: &Path, project: &str, kind: Kind, mc: &str) -> Result<Vec<String>> {
    if kind == Kind::Mods {
        return err("المودات تُثبَّت عبر install_mod");
    }
    let url = format!("{API}/project/{}/version?game_versions={}", enc(project), enc(&format!(r#"["{mc}"]"#)));
    let vs: Vec<Version> = client.get(url).send().await?.error_for_status()?.json().await?;
    let Some(v) = pick_version(&vs) else { return err(format!("لا يوجد إصدار من هذه الحزمة لـ {mc}")) };
    let Some(f) = pick_file(v) else { return err("لا ملف في هذا الإصدار") };
    if !safe_file_name(&f.filename) || !f.filename.ends_with(".zip") {
        return err("اسم ملف غير آمن أو ليس .zip");
    }
    let folder = content::dir(game_dir, kind);
    std::fs::create_dir_all(&folder)?;
    let dest = folder.join(&f.filename);
    if dest.exists() {
        return Ok(Vec::new());
    }
    let job = Job { url: f.url.clone(), dest, sha1: f.hashes.sha1.clone(), size: if f.size > 0 { Some(f.size) } else { None } };
    fetch(client, &job, &Progress::default()).await?;
    Ok(vec![f.filename.clone()])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      {"version_type":"beta","version_number":"2.0-beta","files":[{"url":"https://cdn.modrinth.com/b.jar","filename":"b.jar","primary":true,"size":3,"hashes":{"sha1":"aa"}}],"dependencies":[]},
      {"version_type":"release","version_number":"1.9","files":[
         {"url":"https://cdn.modrinth.com/s.jar","filename":"src.jar","primary":false,"size":1,"hashes":{"sha1":"bb"}},
         {"url":"https://cdn.modrinth.com/r.jar","filename":"r.jar","primary":true,"size":2,"hashes":{"sha1":"cc"}}],
       "dependencies":[{"project_id":"P7dR8mSH","dependency_type":"required"},{"project_id":"x","dependency_type":"optional"}]}
    ]"#;

    #[test]
    fn picks_release_and_primary_file() {
        let vs: Vec<Version> = serde_json::from_str(SAMPLE).unwrap();
        let v = pick_version(&vs).unwrap();
        assert_eq!(v.version_number, "1.9");
        assert_eq!(pick_file(v).unwrap().filename, "r.jar");
        assert_eq!(v.dependencies.iter().filter(|d| d.dependency_type == "required").count(), 1);
        assert_eq!(pick_version(&vs[..1]).unwrap().version_number, "2.0-beta");
    }

    #[test]
    fn urls_are_encoded_and_vanilla_has_none() {
        let u = search_url("fps boost", Loader::Fabric, "1.20.1").unwrap();
        assert!(u.contains("query=fps%20boost") && u.contains("categories%3Afabric"));
        assert!(versions_url("sodium", Loader::Quilt, "1.20.1").unwrap().contains("%5B%22quilt%22%5D"));
        assert!(search_url("x", Loader::Vanilla, "1.20.1").is_none());
        assert!(versions_url("x", Loader::Horizon, "1.8.9").is_none());
    }

    #[test]
    fn curated_slugs_are_safe() {
        for (s, _) in CURATED_FABRIC {
            assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }

    #[test]
    fn browse_needs_a_loader_for_mods_only() {
        let m = browse_url("fps", Kind::Mods, Some(Loader::Forge), Some("1.20.1")).unwrap();
        assert!(m.contains("project_type%3Amod") && m.contains("categories%3Aforge") && m.contains("versions%3A1.20.1"));
        assert!(browse_url("fps", Kind::Mods, None, None).is_none());
        assert!(browse_url("fps", Kind::Mods, Some(Loader::Vanilla), None).is_none());
        let r = browse_url("faithful", Kind::ResourcePacks, None, None).unwrap();
        assert!(r.contains("project_type%3Aresourcepack") && !r.contains("categories"));
        let sh = browse_url("bsl", Kind::ShaderPacks, None, Some("")).unwrap();
        assert!(sh.contains("project_type%3Ashader") && !sh.contains("versions"));
    }
}
