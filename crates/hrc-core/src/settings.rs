//! إعدادات اللانشر العامة (ملف JSON صغير؛ لا خدمات خلفية).
use crate::{err, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub username: String,
    pub default_ram_mb: u32,
    /// مسار Java يدوي (اختياري). إن كان فارغاً يُستخدم Runtime الرسمي تلقائياً.
    pub java_path: Option<String>,
    /// مفتاح CurseForge API الخاص بالمستخدم (اختياري). يُحفظ في settings.json المحلي فقط.
    pub curseforge_key: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { username: "Player".into(), default_ram_mb: 2048, java_path: None, curseforge_key: None }
    }
}

pub fn load(data: &Path) -> Settings {
    std::fs::read(data.join("settings.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// أسماء Minecraft: 3–16 حرفاً من [A-Za-z0-9_].
pub fn valid_username(n: &str) -> bool {
    (3..=16).contains(&n.len()) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn save(data: &Path, s: &Settings) -> Result<()> {
    if !valid_username(&s.username) {
        return err("اسم المستخدم يجب أن يكون 3–16 حرفاً (حروف إنجليزية وأرقام و _)");
    }
    let mut s = s.clone();
    s.default_ram_mb = s.default_ram_mb.clamp(512, 65536);
    s.java_path = s.java_path.filter(|p| !p.trim().is_empty());
    s.curseforge_key = s.curseforge_key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty());
    std::fs::create_dir_all(data)?;
    std::fs::write(data.join("settings.json"), serde_json::to_vec_pretty(&s)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_rules() {
        assert!(valid_username("Steve_01"));
        assert!(!valid_username("ab"));
        assert!(!valid_username("has space"));
        assert!(!valid_username("12345678901234567"));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let d = std::env::temp_dir().join(format!("hrc-set-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let s = Settings { username: "Alex".into(), default_ram_mb: 100, java_path: Some("  ".into()), curseforge_key: Some(" k1 ".into()) };
        save(&d, &s).unwrap();
        let l = load(&d);
        assert_eq!((l.username.as_str(), l.default_ram_mb, l.java_path), ("Alex", 512, None));
        assert_eq!(l.curseforge_key.as_deref(), Some("k1"));
        assert!(save(&d, &Settings { username: "x".into(), ..Default::default() }).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
