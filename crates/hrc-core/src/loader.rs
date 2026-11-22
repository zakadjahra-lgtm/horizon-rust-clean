//! أنواع الـ instance (Vanilla / Fabric / Quilt / Forge / NeoForge / Horizon Client).
//! كل نوع يعيش في instance مستقل بمجلده الخاص: لا تتشارك الأنواع المودات ولا حزم الموارد ولا الـ shaders.
//! Fabric وQuilt يثبَّتان عبر واجهات meta الرسمية (تعيد Version JSON جاهزاً بـ `inheritsFrom`).
use crate::download::{fetch, Job, Progress};
use crate::{err, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    /// Minecraft الأصلية دون أي تعديل.
    #[default]
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    Neoforge,
    /// Minecraft الأصلية + عميل Horizon (HUD عبر Java agent، تجريبي).
    Horizon,
    /// Minecraft الأصلية (Java) بلا أي مود أو حقن كود، مع أدوات Rust في اللانشر (preset الأداء + Rust mods).
    RustTools,
    /// لعبة Rust أصلية تجريبية (ليست Minecraft): بلا Java ولا إصدارات Minecraft ولا مودات.
    Rust,
}

impl Loader {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "vanilla" => Some(Self::Vanilla),
            "fabric" => Some(Self::Fabric),
            "quilt" => Some(Self::Quilt),
            "forge" => Some(Self::Forge),
            "neoforge" => Some(Self::Neoforge),
            "horizon" => Some(Self::Horizon),
            "rusttools" => Some(Self::RustTools),
            "rust" => Some(Self::Rust),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::Neoforge => "NeoForge",
            Self::Horizon => "Horizon Client (beta)",
            Self::RustTools => "Horizon (Rust tools)",
            Self::Rust => "Horizon Rust (experimental)",
        }
    }

    /// هل يدعم هذا النوع مجلد `mods/`؟
    pub fn supports_mods(self) -> bool {
        matches!(self, Self::Fabric | Self::Quilt | Self::Forge | Self::Neoforge)
    }

    /// اسم الـ loader في Modrinth.
    /// الـ loader الفعلي الذي تُحمَّل له المودات: Horizon على 1.13+ يعمل فوق Fabric، وعلى 1.8–1.12.2 بلا مودات خارجية.
    pub fn mods_loader(self, mc: &str) -> Option<Loader> {
        match self {
            Self::Fabric | Self::Quilt | Self::Forge | Self::Neoforge => Some(self),
            Self::Horizon if !crate::perf::legacy_hud_supported(mc) => Some(Self::Fabric),
            _ => None,
        }
    }

    pub fn modrinth_loader(self) -> Option<&'static str> {
        match self {
            Self::Fabric => Some("fabric"),
            Self::Quilt => Some("quilt"),
            Self::Forge => Some("forge"),
            Self::Neoforge => Some("neoforge"),
            _ => None,
        }
    }
}

/// جزء مسار/وسيط آمن (إصدار Minecraft أو loader): حروف وأرقام و . _ + - فقط، بلا `..`.
pub fn safe_part(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !s.contains("..")
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

fn meta_base(l: Loader) -> Option<&'static str> {
    match l {
        Loader::Fabric => Some("https://meta.fabricmc.net/v2"),
        Loader::Quilt => Some("https://meta.quiltmc.org/v3"),
        _ => None,
    }
}

pub fn versions_url(l: Loader, mc: &str) -> Result<String> {
    let base = meta_base(l).ok_or_else(|| crate::Error::Msg(format!("{} غير مدعوم بعد", l.label())))?;
    if !safe_part(mc) {
        return err("إصدار Minecraft غير صالح");
    }
    Ok(format!("{base}/versions/loader/{mc}"))
}

pub fn profile_url(l: Loader, mc: &str, loader_version: &str) -> Result<String> {
    if !safe_part(loader_version) {
        return err("إصدار loader غير صالح");
    }
    Ok(format!("{}/{loader_version}/profile/json", versions_url(l, mc)?))
}

#[derive(Deserialize)]
struct Entry {
    loader: LoaderInfo,
}
#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default)]
    stable: Option<bool>,
}

/// يختار أحدث loader مستقر من القائمة (الأحدث أولاً). Quilt بلا حقل stable فنستبعد beta/alpha/rc بالاسم.
pub fn pick_version(list_json: &str) -> Result<String> {
    let entries: Vec<Entry> = serde_json::from_str(list_json)?;
    let unstable = |v: &str| {
        let v = v.to_ascii_lowercase();
        ["beta", "alpha", "rc", "pre", "snapshot"].iter().any(|t| v.contains(t))
    };
    entries
        .iter()
        .find(|e| e.loader.stable == Some(true))
        .or_else(|| entries.iter().find(|e| e.loader.stable.is_none() && !unstable(&e.loader.version)))
        .or_else(|| entries.first())
        .map(|e| e.loader.version.clone())
        .ok_or_else(|| crate::Error::Msg("لا يوجد loader متوافق مع هذا الإصدار من Minecraft".into()))
}

pub async fn latest_loader_version(client: &reqwest::Client, l: Loader, mc: &str) -> Result<String> {
    let text = get_text(client, &(versions_url(l, mc)?)).await?;
    pick_version(&text)
}

/// يستخرج `id` من profile JSON ويتأكد أنه آمن كاسم مجلد وأنه يرث من إصدار Minecraft المطلوب.
pub fn profile_id(profile_json: &str, mc: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct P {
        id: String,
        #[serde(rename = "inheritsFrom")]
        inherits_from: Option<String>,
    }
    let p: P = serde_json::from_str(profile_json)?;
    if !safe_part(&p.id) {
        return err("معرّف الإصدار في profile غير آمن");
    }
    if p.inherits_from.as_deref() != Some(mc) {
        return err(format!("profile لا يرث من {mc} (يرث من {:?})", p.inherits_from));
    }
    Ok(p.id)
}

/// ينزّل profile JSON الرسمي ويكتبه في `shared/versions/<id>/<id>.json`، ويعيد الـ id.
/// (إصدار Minecraft الأب يجب أن يكون مثبّتاً قبل `install::load_version`.)
pub async fn install_loader_profile(
    client: &reqwest::Client,
    shared: &Path,
    l: Loader,
    mc: &str,
    loader_version: &str,
) -> Result<String> {
    let text = get_text(client, &(profile_url(l, mc, loader_version)?)).await?;
    let id = profile_id(&text, mc)?;
    let dest = shared.join("versions").join(&id).join(format!("{id}.json"));
    if let Some(p) = dest.parent() {
        tokio::fs::create_dir_all(p).await?;
    }
    tokio::fs::write(&dest, text).await?;
    Ok(id)
}

// ───────────── Forge / NeoForge: تشغيل installer الرسمي ─────────────

const FORGE_PROMOS: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_VERSIONS: &str = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

/// Forge الموصى به لهذا الإصدار (وإلا الأحدث) من promotions_slim.json.
pub fn pick_forge(promos_json: &str, mc: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct P {
        promos: std::collections::HashMap<String, String>,
    }
    let p: P = serde_json::from_str(promos_json)?;
    p.promos
        .get(&format!("{mc}-recommended"))
        .or_else(|| p.promos.get(&format!("{mc}-latest")))
        .cloned()
        .ok_or_else(|| Error::Msg(format!("لا يوجد Forge لإصدار {mc}")))
}

/// NeoForge يرقّم إصداراته بحسب Minecraft: 1.21.1 ← `21.1.x` و1.21 ← `21.0.x`.
pub fn neoforge_prefix(mc: &str) -> Option<String> {
    let (major, minor, patch) = crate::perf::parse_mc_version(mc)?;
    match major {
        1 => Some(format!("{minor}.{patch}.")),
        // نظام الترقيم الجديد: Minecraft 26.3 ← NeoForge 26.3.0.x (يطابق installer المرفق neoforge-26.3.0.115)
        m if m >= 26 => Some(format!("{m}.{minor}.{patch}.")),
        _ => None,
    }
}

/// GET نصي مع 3 محاولات (يخفف انقطاعات الشبكة العابرة).
async fn get_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let mut last = None;
    for attempt in 0..3u64 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(800 * attempt)).await;
        }
        match client.get(url).send().await.and_then(|r| r.error_for_status()) {
            Ok(r) => match r.text().await {
                Ok(t) => return Ok(t),
                Err(e) => last = Some(e),
            },
            Err(e) => last = Some(e),
        }
    }
    Err(last.map(Error::from).unwrap_or_else(|| Error::Msg("request failed".into())))
}

fn numeric_key(v: &str) -> Vec<u64> {
    v.split(|c: char| matches!(c, '.' | '-' | '+')).map_while(|p| p.parse::<u64>().ok()).collect()
}

/// أحدث NeoForge مستقر (بلا beta/alpha/rc) يطابق إصدار Minecraft.
pub fn pick_neoforge(versions_json: &str, mc: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct V {
        versions: Vec<String>,
    }
    let v: V = serde_json::from_str(versions_json)?;
    let prefix = neoforge_prefix(mc).ok_or_else(|| Error::Msg(format!("NeoForge لا يدعم {mc}")))?;
    v.versions
        .into_iter()
        .filter(|x| {
            let l = x.to_ascii_lowercase();
            x.starts_with(&prefix) && !["beta", "alpha", "snapshot", "rc"].iter().any(|t| l.contains(t))
        })
        .max_by_key(|x| numeric_key(x))
        .ok_or_else(|| Error::Msg(format!("لا يوجد NeoForge مستقر لإصدار {mc} (يدعم NeoForge من 1.20.2 فما فوق)")))
}

pub fn forge_installer_url(mc: &str, ver: &str) -> Result<String> {
    if !safe_part(mc) || !safe_part(ver) {
        return err("إصدار غير صالح");
    }
    Ok(format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{ver}/forge-{mc}-{ver}-installer.jar"))
}

pub fn neoforge_installer_url(ver: &str) -> Result<String> {
    if !safe_part(ver) {
        return err("إصدار غير صالح");
    }
    Ok(format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{ver}/neoforge-{ver}-installer.jar"))
}

fn version_dirs(shared: &Path) -> HashSet<String> {
    std::fs::read_dir(shared.join("versions"))
        .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default()
}

/// يجد id الإصدار الذي أنشأه الـ installer (اسمه يحوي اسم الـ loader ورقم إصداره، وبه JSON).
fn find_installed_id(shared: &Path, key: &str, ver: &str, before: &HashSet<String>) -> Option<String> {
    let core = ver.split('-').next().unwrap_or(ver);
    let mut all: Vec<String> = version_dirs(shared)
        .into_iter()
        .filter(|n| {
            let l = n.to_ascii_lowercase();
            l.contains(key) && n.contains(core) && shared.join("versions").join(n).join(format!("{n}.json")).is_file()
        })
        .collect();
    all.sort();
    all.iter().find(|n| !before.contains(*n)).or_else(|| all.first()).cloned()
}

/// الـ installer يطبع للـ console، لذا نستخدم java.exe بدل javaw.exe في Windows.
fn console_java(java: &Path) -> PathBuf {
    if java.file_name().and_then(|n| n.to_str()) == Some("javaw.exe") {
        let c = java.with_file_name("java.exe");
        if c.is_file() {
            return c;
        }
    }
    java.to_path_buf()
}

/// ينزّل installer الرسمي (مع التحقق من SHA1 المنشور في Maven) ويشغّله بـ `--installClient`
/// داخل `shared/` (نفس تخطيط Mojang)، ويعيد id الإصدار الناتج.
pub async fn run_installer(client: &reqwest::Client, shared: &Path, l: Loader, mc: &str, ver: &str, java: &Path) -> Result<String> {
    let (url, file, key) = match l {
        Loader::Forge => (forge_installer_url(mc, ver)?, format!("forge-{mc}-{ver}-installer.jar"), "forge"),
        Loader::Neoforge => (neoforge_installer_url(ver)?, format!("neoforge-{ver}-installer.jar"), "neoforge"),
        _ => return err("هذا النوع لا يستخدم installer"),
    };
    let sha1_text = get_text(client, &(format!("{url}.sha1"))).await?;
    let sha1 = sha1_text.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
    if sha1.len() != 40 || !sha1.chars().all(|c| c.is_ascii_hexdigit()) {
        return err("تعذّر الحصول على SHA1 رسمي للـ installer");
    }
    let dest = shared.join("installers").join(&file);
    fetch(client, &Job { url, dest: dest.clone(), sha1: Some(sha1), size: None }, &Progress::default()).await?;

    tokio::fs::create_dir_all(shared).await?;
    let lp = shared.join("launcher_profiles.json");
    if !lp.exists() {
        tokio::fs::write(&lp, br#"{"profiles":{}}"#).await?;
    }
    let before = version_dirs(shared);

    let mut cmd = tokio::process::Command::new(console_java(java));
    cmd.args(["-Djava.awt.headless=true", "-jar"])
        .arg(&dest)
        .arg("--installClient")
        .arg(shared)
        .current_dir(shared)
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let out = tokio::time::timeout(std::time::Duration::from_secs(1200), cmd.output())
        .await
        .map_err(|_| Error::Msg("انتهت مهلة installer (20 دقيقة)".into()))??;
    if !out.status.success() {
        let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        let tail: Vec<&str> = text.lines().rev().take(15).collect::<Vec<_>>().into_iter().rev().collect();
        return err(format!("فشل installer ({}):\n{}", out.status, tail.join("\n")));
    }
    find_installed_id(shared, key, ver, &before).ok_or_else(|| Error::Msg("انتهى installer لكن لم أجد الإصدار الناتج".into()))
}

/// نتيجة تثبيت نوع: إصدار الـ loader (إن وُجد) و`launch_id` = معرّف الإصدار الذي يُشغَّل فعلاً.
#[derive(Debug, Clone)]
pub struct Installed {
    pub loader_version: Option<String>,
    pub launch_id: String,
}

/// يثبّت ملفات الـ loader فوق إصدار Minecraft (يجب أن يكون JSON وjar الأصلي مثبّتين قبلها).
/// Horizon: للإصدارات 1.8–1.12.2 يبقى Vanilla + agent، وللأحدث يستخدم Fabric كقاعدة.
pub async fn install(client: &reqwest::Client, shared: &Path, l: Loader, mc: &str, java: &Path) -> Result<Installed> {
    let vanilla = || Installed { loader_version: None, launch_id: mc.to_string() };
    match l {
        Loader::Rust => err("Horizon Rust is a native game: nothing to install from Mojang"),
        Loader::Vanilla | Loader::RustTools => Ok(vanilla()),
        Loader::Horizon if crate::perf::legacy_hud_supported(mc) => Ok(vanilla()),
        Loader::Fabric | Loader::Quilt | Loader::Horizon => {
            let base = if l == Loader::Quilt { Loader::Quilt } else { Loader::Fabric };
            let v = latest_loader_version(client, base, mc).await?;
            let id = install_loader_profile(client, shared, base, mc, &v).await?;
            Ok(Installed { loader_version: Some(v), launch_id: id })
        }
        Loader::Forge => {
            let text = get_text(client, &(FORGE_PROMOS)).await?;
            let v = pick_forge(&text, mc)?;
            let id = run_installer(client, shared, l, mc, &v, java).await?;
            Ok(Installed { loader_version: Some(v), launch_id: id })
        }
        Loader::Neoforge => {
            let text = get_text(client, &(NEOFORGE_VERSIONS)).await?;
            let v = pick_neoforge(&text, mc)?;
            let id = run_installer(client, shared, l, mc, &v, java).await?;
            Ok(Installed { loader_version: Some(v), launch_id: id })
        }
    }
}

/// هل يحتاج هذا النوع تثبيت Minecraft الأصلية كاملةً قبل تشغيل الـ installer؟
pub fn needs_vanilla_first(l: Loader) -> bool {
    matches!(l, Loader::Forge | Loader::Neoforge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_capabilities() {
        assert_eq!(Loader::parse(" NeoForge "), Some(Loader::Neoforge));
        assert_eq!(Loader::parse("liteloader"), None);
        assert_eq!(Loader::Horizon.mods_loader("1.20.1"), Some(Loader::Fabric));
        assert_eq!(Loader::Horizon.mods_loader("1.8.9"), None);
        assert_eq!(Loader::Vanilla.mods_loader("1.20.1"), None);
        assert_eq!(Loader::parse("rust"), Some(Loader::Rust));
        assert_eq!(Loader::parse("rusttools"), Some(Loader::RustTools));
        assert_eq!(Loader::RustTools.mods_loader("1.20.1"), None);
        assert!(!Loader::RustTools.supports_mods());
        assert_eq!(Loader::Rust.mods_loader("1.20.1"), None);
        assert!(!Loader::Rust.supports_mods() && Loader::Rust.modrinth_loader().is_none());
        assert!(Loader::Fabric.supports_mods() && !Loader::Vanilla.supports_mods() && !Loader::Horizon.supports_mods());
        assert_eq!(Loader::Quilt.modrinth_loader(), Some("quilt"));
        assert_eq!(Loader::Vanilla.modrinth_loader(), None);
    }

    #[test]
    fn urls_and_safety() {
        assert_eq!(
            profile_url(Loader::Fabric, "1.20.1", "0.16.9").unwrap(),
            "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.16.9/profile/json"
        );
        assert!(versions_url(Loader::Quilt, "1.20.1").unwrap().starts_with("https://meta.quiltmc.org/v3/"));
        assert!(versions_url(Loader::Vanilla, "1.20.1").is_err());
        assert!(versions_url(Loader::Fabric, "../x").is_err());
        assert!(profile_url(Loader::Fabric, "1.20.1", "a/b").is_err());
        assert!(!safe_part("") && !safe_part("a b") && !safe_part("a..b") && safe_part("1.20.1-pre+3"));
    }

    #[test]
    fn picks_stable_fabric_and_non_beta_quilt() {
        let fabric = r#"[{"loader":{"version":"0.17.0-beta.1","stable":false}},{"loader":{"version":"0.16.9","stable":true}}]"#;
        assert_eq!(pick_version(fabric).unwrap(), "0.16.9");
        let quilt = r#"[{"loader":{"version":"0.27.0-beta.1"}},{"loader":{"version":"0.26.4"}}]"#;
        assert_eq!(pick_version(quilt).unwrap(), "0.26.4");
        assert!(pick_version("[]").is_err());
    }

    #[test]
    fn profile_id_checks() {
        let ok = r#"{"id":"fabric-loader-0.16.9-1.20.1","inheritsFrom":"1.20.1"}"#;
        assert_eq!(profile_id(ok, "1.20.1").unwrap(), "fabric-loader-0.16.9-1.20.1");
        assert!(profile_id(ok, "1.19.4").is_err());
        assert!(profile_id(r#"{"id":"../evil","inheritsFrom":"1.20.1"}"#, "1.20.1").is_err());
        assert!(profile_id(r#"{"id":"x"}"#, "1.20.1").is_err());
    }

    #[test]
    fn forge_pick_and_urls() {
        let j = r#"{"promos":{"1.20.1-latest":"47.3.12","1.20.1-recommended":"47.3.0","1.8.9-latest":"11.15.1.2318-1.8.9"}}"#;
        assert_eq!(pick_forge(j, "1.20.1").unwrap(), "47.3.0");
        assert_eq!(pick_forge(j, "1.8.9").unwrap(), "11.15.1.2318-1.8.9");
        assert!(pick_forge(j, "1.7.2").is_err());
        assert_eq!(
            forge_installer_url("1.20.1", "47.3.0").unwrap(),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.3.0/forge-1.20.1-47.3.0-installer.jar"
        );
        assert!(forge_installer_url("1.20.1", "../x").is_err());
    }

    #[test]
    fn neoforge_pick_numeric_and_stable() {
        let j = r#"{"isSnapshot":false,"versions":["21.1.9","21.1.172","21.1.100","21.1.200-beta","21.0.5","20.4.237"]}"#;
        assert_eq!(neoforge_prefix("1.21.1").as_deref(), Some("21.1."));
        assert_eq!(neoforge_prefix("1.21").as_deref(), Some("21.0."));
        assert_eq!(neoforge_prefix("24w01a"), None);
        assert_eq!(neoforge_prefix("26.3").as_deref(), Some("26.3.0."));
        assert_eq!(pick_neoforge(r#"{"versions":["26.3.0.115","26.3.1.2","26.2.0.9"]}"#, "26.3").unwrap(), "26.3.0.115");
        assert_eq!(pick_neoforge(j, "1.21.1").unwrap(), "21.1.172"); // 172 > 100 > 9 رقمياً، وبلا beta
        assert_eq!(pick_neoforge(j, "1.21").unwrap(), "21.0.5");
        assert!(pick_neoforge(j, "1.20.1").is_err());
        assert_eq!(
            neoforge_installer_url("21.1.172").unwrap(),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.172/neoforge-21.1.172-installer.jar"
        );
    }

    #[test]
    fn finds_installer_output_dir() {
        let d = std::env::temp_dir().join(format!("hrc-ldr-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for n in ["1.20.1", "1.20.1-forge-47.3.0", "fabric-loader-0.16.9-1.20.1"] {
            std::fs::create_dir_all(d.join("versions").join(n)).unwrap();
            std::fs::write(d.join("versions").join(n).join(format!("{n}.json")), "{}").unwrap();
        }
        let before: HashSet<String> = ["1.20.1".to_string()].into_iter().collect();
        assert_eq!(find_installed_id(&d, "forge", "47.3.0", &before).as_deref(), Some("1.20.1-forge-47.3.0"));
        assert_eq!(find_installed_id(&d, "neoforge", "21.1.1", &before), None);
        let _ = std::fs::remove_dir_all(&d);
    }
}
