//! Profiles: مجموعة إعدادات للعميل (أي مودات HUD مفعّلة وأين، وpreset الأداء).
//! تُحفظ في `<data>/profiles/<uuid>.json`، وتُترجم قبل التشغيل إلى `horizon-client.properties`
//! الذي يقرؤه الـ Java agent. كل المودات مطفأة افتراضياً (Clean).
use crate::instance::valid_id;
use crate::perf::{legacy_hud_supported, PerfPreset};
use crate::{err, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// معرّفات المودات المنفّذة فعلاً في `client/` (يجب أن تطابق مفاتيح `Overlay.java`).
pub const MOD_IDS: [&str; 3] = ["fps", "cps", "keystrokes"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HudMod {
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// 0 = تلقائي حسب دقة النافذة.
    #[serde(default)]
    pub hud_scale: u32,
    #[serde(default)]
    pub perf: PerfPreset,
    /// Rust mods (beta) تعمل داخل اللانشر قبل التشغيل؛ مطفأة افتراضياً.
    #[serde(default)]
    pub rust_mods: BTreeMap<String, bool>,
    pub mods: BTreeMap<String, HudMod>,
}

fn default_pos(mod_id: &str) -> (i32, i32) {
    match mod_id {
        "fps" => (6, 6),
        "cps" => (6, 26),
        _ => (6, 50), // keystrokes
    }
}

impl Profile {
    /// Profile نظيف: كل المودات مطفأة، وVanilla للأداء.
    pub fn clean(id: String, name: String) -> Profile {
        let mods = MOD_IDS
            .iter()
            .map(|m| {
                let (x, y) = default_pos(m);
                (m.to_string(), HudMod { enabled: false, x, y })
            })
            .collect();
        Profile { id, name, hud_scale: 0, perf: PerfPreset::Vanilla, rust_mods: BTreeMap::new(), mods }
    }
}

pub fn profiles_dir(data: &Path) -> PathBuf {
    data.join("profiles")
}

fn path(data: &Path, id: &str) -> PathBuf {
    profiles_dir(data).join(format!("{id}.json"))
}

fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 64 {
        return err("اسم الـ profile يجب أن يكون بين 1 و64 حرفاً");
    }
    Ok(n.to_string())
}

/// يتحقق ويصحّح: مودات غير معروفة ترفض، والإحداثيات تُقيَّد، والناقص يُكمَّل بالافتراضي.
fn normalize(mut p: Profile) -> Result<Profile> {
    if !valid_id(&p.id) {
        return err("معرّف profile غير صالح");
    }
    p.name = clean_name(&p.name)?;
    if let Some(bad) = p.mods.keys().find(|k| !MOD_IDS.contains(&k.as_str())) {
        return err(format!("مود غير معروف: {bad}"));
    }
    if let Some(bad) = p.rust_mods.keys().find(|k| !crate::rustmods::IDS.contains(&k.as_str())) {
        return err(format!("Rust mod غير معروف: {bad}"));
    }
    for id in crate::rustmods::IDS {
        p.rust_mods.entry(id.to_string()).or_insert(false);
    }
    p.hud_scale = p.hud_scale.min(8);
    for m in MOD_IDS {
        let (dx, dy) = default_pos(m);
        let e = p.mods.entry(m.to_string()).or_insert(HudMod { enabled: false, x: dx, y: dy });
        e.x = e.x.clamp(0, 4000);
        e.y = e.y.clamp(0, 4000);
    }
    Ok(p)
}

pub fn load(data: &Path, id: &str) -> Result<Profile> {
    if !valid_id(id) {
        return err("معرّف profile غير صالح");
    }
    normalize(serde_json::from_slice(&std::fs::read(path(data, id))?)?)
}

pub fn save(data: &Path, p: &Profile) -> Result<Profile> {
    let p = normalize(p.clone())?;
    std::fs::create_dir_all(profiles_dir(data))?;
    std::fs::write(path(data, &p.id), serde_json::to_vec_pretty(&p)?)?;
    Ok(p)
}

pub fn create(data: &Path, name: &str) -> Result<Profile> {
    let p = Profile::clean(uuid::Uuid::new_v4().to_string(), clean_name(name)?);
    save(data, &p)
}

pub fn list(data: &Path) -> Result<Vec<Profile>> {
    let dir = profiles_dir(data);
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for e in std::fs::read_dir(dir)? {
        let name = e?.file_name().to_string_lossy().into_owned();
        if let Some(id) = name.strip_suffix(".json") {
            if let Ok(p) = load(data, id) {
                out.push(p);
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn rename(data: &Path, id: &str, name: &str) -> Result<Profile> {
    let mut p = load(data, id)?;
    p.name = clean_name(name)?;
    save(data, &p)
}

pub fn duplicate(data: &Path, id: &str) -> Result<Profile> {
    let mut p = load(data, id)?;
    p.id = uuid::Uuid::new_v4().to_string();
    p.name = format!("{} (copy)", p.name.chars().take(55).collect::<String>());
    save(data, &p)
}

pub fn delete(data: &Path, id: &str) -> Result<()> {
    if !valid_id(id) {
        return err("معرّف profile غير صالح");
    }
    let f = path(data, id);
    if f.exists() {
        std::fs::remove_file(f)?;
    }
    Ok(())
}

/// نص `horizon-client.properties` (ترتيب ثابت، يطابق مفاتيح `Overlay.java`/`Config.java`).
pub fn to_properties(p: &Profile) -> String {
    let mut s = String::from("# generated by Horizon Rust Clean — do not edit (overwritten at launch)\n");
    if p.hud_scale > 0 {
        s.push_str(&format!("hud.scale={}\n", p.hud_scale));
    }
    for (id, m) in &p.mods {
        s.push_str(&format!("{id}.enabled={}\n{id}.x={}\n{id}.y={}\n", m.enabled, m.x, m.y));
    }
    s
}

pub fn write_client_config(game_dir: &Path, p: &Profile) -> Result<()> {
    std::fs::create_dir_all(game_dir)?;
    std::fs::write(game_dir.join("horizon-client.properties"), to_properties(p))?;
    Ok(())
}

/// وسيط `-javaagent` إن كان الإصدار مدعوماً (LWJGL 2) وكان ملف الـ agent موجوداً، وإلا None.
pub fn agent_arg(agent_jar: &Path, game_dir: &Path, mc_version: &str) -> Option<String> {
    if !legacy_hud_supported(mc_version) || !agent_jar.is_file() {
        return None;
    }
    Some(format!("-javaagent:{}={}", agent_jar.display(), game_dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hrc-prof-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn clean_profile_has_everything_off() {
        let d = tmp("a");
        let p = create(&d, "  Default  ").unwrap();
        assert_eq!(p.name, "Default");
        assert_eq!(p.perf, PerfPreset::Vanilla);
        assert_eq!(p.mods.len(), MOD_IDS.len());
        assert!(p.mods.values().all(|m| !m.enabled));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn save_load_roundtrip_and_clamp() {
        let d = tmp("b");
        let mut p = create(&d, "PvP").unwrap();
        p.mods.get_mut("fps").unwrap().enabled = true;
        p.mods.get_mut("fps").unwrap().x = 99999; // خارج الحد
        p.hud_scale = 50;
        p.perf = PerfPreset::Balanced;
        save(&d, &p).unwrap();
        let l = load(&d, &p.id).unwrap();
        assert!(l.mods["fps"].enabled);
        assert_eq!(l.mods["fps"].x, 4000);
        assert_eq!(l.hud_scale, 8);
        assert_eq!(l.perf, PerfPreset::Balanced);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn rejects_unknown_mod_bad_id_and_empty_name() {
        let d = tmp("c");
        let mut p = create(&d, "X").unwrap();
        p.mods.insert("freecam".into(), HudMod { enabled: true, x: 0, y: 0 });
        assert!(save(&d, &p).is_err());
        assert!(load(&d, "../../etc/passwd").is_err());
        assert!(delete(&d, "..").is_err());
        assert!(create(&d, "   ").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn duplicate_rename_list_delete() {
        let d = tmp("d");
        let a = create(&d, "Beta").unwrap();
        let b = duplicate(&d, &a.id).unwrap();
        assert_ne!(a.id, b.id);
        assert!(b.name.ends_with("(copy)"));
        rename(&d, &a.id, "Alpha").unwrap();
        let names: Vec<String> = list(&d).unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["Alpha".to_string(), "Beta (copy)".to_string()]);
        delete(&d, &a.id).unwrap();
        assert_eq!(list(&d).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn properties_match_what_the_agent_reads() {
        let mut p = Profile::clean("00000000-0000-0000-0000-000000000001".into(), "P".into());
        p.mods.get_mut("cps").unwrap().enabled = true;
        p.hud_scale = 2;
        let s = to_properties(&p);
        assert!(s.contains("cps.enabled=true\ncps.x=6\ncps.y=26\n"));
        assert!(s.contains("fps.enabled=false\n"));
        assert!(s.contains("hud.scale=2\n"));
        assert!(!to_properties(&Profile::clean("00000000-0000-0000-0000-000000000002".into(), "Q".into())).contains("hud.scale"));
    }

    #[test]
    fn write_config_and_agent_arg() {
        let d = tmp("e");
        let p = Profile::clean("00000000-0000-0000-0000-000000000003".into(), "P".into());
        let game = d.join("game");
        write_client_config(&game, &p).unwrap();
        assert!(game.join("horizon-client.properties").is_file());
        let jar = d.join("horizon-client.jar");
        assert_eq!(agent_arg(&jar, &game, "1.8.9"), None); // الملف غير موجود
        std::fs::write(&jar, b"x").unwrap();
        assert!(agent_arg(&jar, &game, "1.8.9").unwrap().starts_with("-javaagent:"));
        assert_eq!(agent_arg(&jar, &game, "1.20.1"), None); // LWJGL 3: غير مدعوم بعد
        let _ = std::fs::remove_dir_all(&d);
    }
}
