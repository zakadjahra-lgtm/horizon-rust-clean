//! فحص توافق ملف مود (.jar) مع الـ Loader وإصدار Minecraft للـ instance، بقراءة الـ metadata داخل الـ jar.
//! نحدّد نوع المود (Fabric/Quilt/Forge/NeoForge) ونفحص نطاق إصدار Minecraft لمودات Fabric وQuilt.
//! لمودات Forge/NeoForge نفحص نوع الـ loader فقط (لا نحلل نطاقات mods.toml) — وهذا مذكور في STATUS.
use crate::loader::Loader;
use crate::perf::parse_mc_version;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModKind {
    Fabric,
    Quilt,
    Forge,
    Neoforge,
}

impl ModKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::Neoforge => "NeoForge",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModMeta {
    pub kinds: Vec<ModKind>,
    /// نطاقات إصدار Minecraft (أي واحد منها كافٍ). None = غير معروف.
    pub mc_ranges: Option<Vec<String>>,
}

type V = (u32, u32, u32);

fn cmp_ok(op: &str, v: V, t: V) -> bool {
    match op {
        ">=" => v >= t,
        ">" => v > t,
        "<=" => v <= t,
        "<" => v < t,
        _ => v == t,
    }
}

/// يحقق شرطاً واحداً (بلا مسافات) مثل `>=1.20`, `~1.20.1`, `1.20.x`, `*`. None = صيغة غير مفهومة.
fn atom(a: &str, v: V) -> Option<bool> {
    if a == "*" || a.is_empty() {
        return Some(true);
    }
    for op in [">=", "<=", ">", "<", "="] {
        if let Some(r) = a.strip_prefix(op) {
            return Some(cmp_ok(op, v, parse_loose(r)?));
        }
    }
    if let Some(r) = a.strip_prefix('~') {
        let t = parse_loose(r)?;
        return Some(v >= t && v < (t.0, t.1 + 1, 0));
    }
    if let Some(r) = a.strip_prefix('^') {
        let t = parse_loose(r)?;
        return Some(v >= t && v < (t.0 + 1, 0, 0));
    }
    if a.contains('x') || a.contains('X') || a.contains('*') {
        let p: Vec<&str> = a.split('.').collect();
        let want = [v.0, v.1, v.2];
        for (i, seg) in p.iter().enumerate().take(3) {
            if matches!(*seg, "x" | "X" | "*") {
                break;
            }
            if seg.parse::<u32>().ok()? != want[i] {
                return Some(false);
            }
        }
        return Some(true);
    }
    Some(v == parse_loose(a)?)
}

fn parse_loose(s: &str) -> Option<V> {
    let p: Vec<&str> = s.split('.').collect();
    if p.is_empty() || p.len() > 3 {
        return None;
    }
    let n: Option<Vec<u32>> = p.iter().map(|x| x.parse().ok()).collect();
    let n = n?;
    Some((n[0], n.get(1).copied().unwrap_or(0), n.get(2).copied().unwrap_or(0)))
}

/// هل `mc` يحقق أحد النطاقات؟ Some(true/false)، أو None إن لم نفهم الصيغة أو الإصدار (لا تحذير في هذه الحالة).
pub fn mc_satisfies(ranges: &[String], mc: &str) -> Option<bool> {
    let (a, b, c) = parse_mc_version(mc)?;
    let v = (a, b, c);
    let mut unknown = false;
    for r in ranges {
        let mut all = Some(true);
        for part in r.split_whitespace().filter(|p| !p.is_empty()) {
            match atom(part, v) {
                Some(true) => {}
                Some(false) => {
                    all = Some(false);
                    break;
                }
                None => {
                    all = None;
                    break;
                }
            }
        }
        match all {
            Some(true) => return Some(true),
            Some(false) => {}
            None => unknown = true,
        }
    }
    if unknown { None } else { Some(false) }
}

fn read_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut f = zip.by_name(name).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    Some(s)
}

fn string_or_list(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Array(a) => Some(a.iter().filter_map(|x| x.as_str().map(String::from)).collect()),
        _ => None,
    }
}

pub fn read_meta(jar: &Path) -> Option<ModMeta> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(jar).ok()?).ok()?;
    let mut kinds = Vec::new();
    let mut mc_ranges = None;
    if let Some(t) = read_entry(&mut zip, "fabric.mod.json") {
        kinds.push(ModKind::Fabric);
        if let Ok(j) = serde_json::from_str::<Value>(&t) {
            mc_ranges = j.get("depends").and_then(|d| d.get("minecraft")).and_then(string_or_list);
        }
    }
    if let Some(t) = read_entry(&mut zip, "quilt.mod.json") {
        kinds.push(ModKind::Quilt);
        if let Ok(j) = serde_json::from_str::<Value>(&t) {
            // quilt_loader.depends: [{id:"minecraft", versions:"..."}] أو سلاسل
            if let Some(deps) = j.pointer("/quilt_loader/depends").and_then(|d| d.as_array()) {
                for d in deps {
                    if d.get("id").and_then(|x| x.as_str()) == Some("minecraft") {
                        mc_ranges = d.get("versions").and_then(string_or_list);
                    }
                }
            }
        }
    }
    if zip.by_name("META-INF/neoforge.mods.toml").is_ok() {
        kinds.push(ModKind::Neoforge);
    }
    if zip.by_name("META-INF/mods.toml").is_ok() || zip.by_name("mcmod.info").is_ok() {
        kinds.push(ModKind::Forge);
    }
    Some(ModMeta { kinds, mc_ranges })
}

/// ما يقبله كل loader من أنواع المودات. Quilt يحمّل مودات Fabric أيضاً؛ NeoForge 1.20.1 يقرأ mods.toml.
fn accepts(loader: Loader, k: ModKind) -> bool {
    match loader {
        Loader::Fabric => k == ModKind::Fabric,
        Loader::Quilt => matches!(k, ModKind::Quilt | ModKind::Fabric),
        Loader::Forge => k == ModKind::Forge,
        Loader::Neoforge => matches!(k, ModKind::Neoforge | ModKind::Forge),
        _ => false,
    }
}

/// رسالة تحذير واضحة أو None إن كان المود متوافقاً (أو لم نستطع الحكم).
pub fn check(jar: &Path, loader: Loader, mc: &str) -> Option<String> {
    let meta = read_meta(jar)?;
    if meta.kinds.is_empty() {
        return Some("No mod metadata found in this file — it may not be a mod.".into());
    }
    if !meta.kinds.iter().any(|k| accepts(loader, *k)) {
        let have: Vec<&str> = meta.kinds.iter().map(|k| k.label()).collect();
        return Some(format!("This is a {} mod; it will not load in a {} instance.", have.join("/"), loader.label()));
    }
    if matches!(loader, Loader::Fabric | Loader::Quilt) {
        if let Some(r) = &meta.mc_ranges {
            if mc_satisfies(r, mc) == Some(false) {
                return Some(format!("This mod requires Minecraft {} but this instance uses {mc}.", r.join(" or ")));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn r(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn ranges() {
        assert_eq!(mc_satisfies(&r(&["1.20.1"]), "1.20.1"), Some(true));
        assert_eq!(mc_satisfies(&r(&["1.20.1"]), "1.20.2"), Some(false));
        assert_eq!(mc_satisfies(&r(&[">=1.20 <1.21"]), "1.20.4"), Some(true));
        assert_eq!(mc_satisfies(&r(&[">=1.20 <1.21"]), "1.21"), Some(false));
        assert_eq!(mc_satisfies(&r(&["~1.20.1"]), "1.20.6"), Some(true));
        assert_eq!(mc_satisfies(&r(&["~1.20.1"]), "1.21"), Some(false));
        assert_eq!(mc_satisfies(&r(&["~1.20.1"]), "1.20.0"), Some(false));
        assert_eq!(mc_satisfies(&r(&["^1.19"]), "1.20.1"), Some(true));
        assert_eq!(mc_satisfies(&r(&["1.20.x"]), "1.20.4"), Some(true));
        assert_eq!(mc_satisfies(&r(&["1.20.x"]), "1.19.4"), Some(false));
        assert_eq!(mc_satisfies(&r(&["*"]), "1.8.9"), Some(true));
        assert_eq!(mc_satisfies(&r(&["1.19.2", "1.20.1"]), "1.20.1"), Some(true)); // OR
        assert_eq!(mc_satisfies(&r(&[">=26.1"]), "26.3"), Some(true));
        assert_eq!(mc_satisfies(&r(&["weird!"]), "1.20.1"), None);
        assert_eq!(mc_satisfies(&r(&["1.20.1"]), "24w14a"), None);
    }

    fn jar(tag: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("hrc-mc-{tag}-{}.jar", std::process::id()));
        let mut z = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
        for (n, c) in files {
            z.start_file(*n, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(c.as_bytes()).unwrap();
        }
        z.finish().unwrap();
        p
    }

    #[test]
    fn loader_and_version_checks() {
        let fab = jar("f", &[("fabric.mod.json", r#"{"id":"x","depends":{"minecraft":"~1.20.1"}}"#)]);
        assert_eq!(check(&fab, Loader::Fabric, "1.20.1"), None);
        assert_eq!(check(&fab, Loader::Quilt, "1.20.4"), None); // Quilt يقبل Fabric
        assert!(check(&fab, Loader::Fabric, "1.19.2").unwrap().contains("requires Minecraft"));
        assert!(check(&fab, Loader::Forge, "1.20.1").unwrap().contains("will not load in a Forge"));

        let forge = jar("g", &[("META-INF/mods.toml", "modLoader=\"javafml\"")]);
        assert!(check(&forge, Loader::Fabric, "1.20.1").unwrap().contains("Forge mod"));
        assert_eq!(check(&forge, Loader::Forge, "1.20.1"), None);

        let quilt = jar("q", &[("quilt.mod.json", r#"{"quilt_loader":{"depends":[{"id":"minecraft","versions":">=1.20"}]}}"#)]);
        assert!(check(&quilt, Loader::Fabric, "1.20.1").unwrap().contains("Quilt mod"));
        assert_eq!(check(&quilt, Loader::Quilt, "1.20.1"), None);

        let none = jar("n", &[("readme.txt", "x")]);
        assert!(check(&none, Loader::Fabric, "1.20.1").unwrap().contains("No mod metadata"));
        for p in [fab, forge, quilt, none] {
            let _ = std::fs::remove_file(p);
        }
    }
}
