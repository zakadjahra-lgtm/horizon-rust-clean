//! محتوى كل instance: المودات وحزم الموارد وحزم الـ shaders. كل instance له مجلدات خاصة به،
//! فلا يتشارك Vanilla وFabric وForge… أي ملف. المود يُطفأ بإعادة تسميته `.jar.disabled` (بلا حذف).
use crate::loader::Loader;
use crate::{err, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mods,
    ResourcePacks,
    ShaderPacks,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        match s {
            "mods" => Some(Kind::Mods),
            "resourcepacks" => Some(Kind::ResourcePacks),
            "shaderpacks" => Some(Kind::ShaderPacks),
            _ => None,
        }
    }
    pub fn dir_name(self) -> &'static str {
        match self {
            Kind::Mods => "mods",
            Kind::ResourcePacks => "resourcepacks",
            Kind::ShaderPacks => "shaderpacks",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Item {
    /// الاسم الأساسي (للمودات بدون اللاحقة `.disabled`).
    pub name: String,
    pub enabled: bool,
    pub size: u64,
    pub is_dir: bool,
}

pub fn dir(game_dir: &Path, kind: Kind) -> PathBuf {
    game_dir.join(kind.dir_name())
}

/// ينشئ مجلدات المحتوى المناسبة لنوع الـ instance (المودات فقط لمن يدعمها).
pub fn ensure_dirs(game_dir: &Path, loader: Loader) -> Result<()> {
    std::fs::create_dir_all(dir(game_dir, Kind::ResourcePacks))?;
    std::fs::create_dir_all(dir(game_dir, Kind::ShaderPacks))?;
    if loader.supports_mods() || loader == Loader::Horizon {
        std::fs::create_dir_all(dir(game_dir, Kind::Mods))?;
    }
    Ok(())
}

/// اسم ملف واحد فقط: بلا فواصل مسار ولا `..` ولا محارف يرفضها Windows.
pub fn safe_file_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 200
        && n != "."
        && !n.contains("..")
        && !n.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'))
}

pub fn list(game_dir: &Path, kind: Kind) -> Result<Vec<Item>> {
    let d = dir(game_dir, kind);
    let mut out = Vec::new();
    if !d.exists() {
        return Ok(out);
    }
    for e in std::fs::read_dir(&d)? {
        let e = e?;
        let file = e.file_name().to_string_lossy().into_owned();
        let meta = e.metadata()?;
        let is_dir = meta.is_dir();
        if kind == Kind::Mods {
            if is_dir {
                continue;
            }
            if let Some(base) = file.strip_suffix(".jar.disabled") {
                out.push(Item { name: format!("{base}.jar"), enabled: false, size: meta.len(), is_dir: false });
            } else if file.ends_with(".jar") {
                out.push(Item { name: file, enabled: true, size: meta.len(), is_dir: false });
            }
        } else if is_dir || file.ends_with(".zip") {
            out.push(Item { name: file, enabled: true, size: if is_dir { 0 } else { meta.len() }, is_dir });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// تشغيل/إطفاء مود (المودات فقط). `name` هو الاسم الأساسي `x.jar`.
pub fn set_mod_enabled(game_dir: &Path, name: &str, enabled: bool) -> Result<()> {
    if !safe_file_name(name) || !name.ends_with(".jar") {
        return err("اسم مود غير صالح");
    }
    let on = dir(game_dir, Kind::Mods).join(name);
    let off = dir(game_dir, Kind::Mods).join(format!("{name}.disabled"));
    let (from, to) = if enabled { (off, on) } else { (on, off) };
    if to.exists() && !from.exists() {
        return Ok(()); // بالحالة المطلوبة أصلاً
    }
    if !from.exists() {
        return err("المود غير موجود");
    }
    std::fs::rename(from, to)?;
    Ok(())
}

/// يحذف عنصراً من مجلد المحتوى (للمود: يحذف الحالتين).
pub fn delete(game_dir: &Path, kind: Kind, name: &str) -> Result<()> {
    if !safe_file_name(name) {
        return err("اسم غير صالح");
    }
    let d = dir(game_dir, kind);
    let mut targets = vec![d.join(name)];
    if kind == Kind::Mods {
        targets.push(d.join(format!("{name}.disabled")));
    }
    let mut found = false;
    for t in targets {
        if t.is_dir() {
            std::fs::remove_dir_all(&t)?;
            found = true;
        } else if t.is_file() {
            std::fs::remove_file(&t)?;
            found = true;
        }
    }
    if found { Ok(()) } else { err("العنصر غير موجود") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hrc-cnt-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn dirs_depend_on_loader() {
        let d = tmp("a");
        ensure_dirs(&d.join("v"), Loader::Vanilla).unwrap();
        ensure_dirs(&d.join("f"), Loader::Fabric).unwrap();
        assert!(d.join("v/resourcepacks").is_dir() && d.join("v/shaderpacks").is_dir());
        assert!(!d.join("v/mods").exists());
        assert!(d.join("f/mods").is_dir());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn toggle_list_delete_mods() {
        let d = tmp("b");
        ensure_dirs(&d, Loader::Fabric).unwrap();
        std::fs::write(d.join("mods/sodium.jar"), b"x").unwrap();
        std::fs::write(d.join("mods/readme.txt"), b"x").unwrap(); // يُتجاهل
        let l = list(&d, Kind::Mods).unwrap();
        assert_eq!(l, vec![Item { name: "sodium.jar".into(), enabled: true, size: 1, is_dir: false }]);
        set_mod_enabled(&d, "sodium.jar", false).unwrap();
        assert!(d.join("mods/sodium.jar.disabled").is_file() && !d.join("mods/sodium.jar").exists());
        let l = list(&d, Kind::Mods).unwrap();
        assert_eq!((l[0].name.as_str(), l[0].enabled), ("sodium.jar", false));
        set_mod_enabled(&d, "sodium.jar", false).unwrap(); // تكرار آمن
        set_mod_enabled(&d, "sodium.jar", true).unwrap();
        assert!(d.join("mods/sodium.jar").is_file());
        delete(&d, Kind::Mods, "sodium.jar").unwrap();
        assert!(list(&d, Kind::Mods).unwrap().is_empty());
        assert!(delete(&d, Kind::Mods, "sodium.jar").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn packs_list_zip_and_folders() {
        let d = tmp("c");
        ensure_dirs(&d, Loader::Vanilla).unwrap();
        std::fs::write(d.join("resourcepacks/faithful.zip"), b"zz").unwrap();
        std::fs::create_dir_all(d.join("resourcepacks/MyPack")).unwrap();
        std::fs::write(d.join("resourcepacks/notes.txt"), b"x").unwrap();
        let names: Vec<String> = list(&d, Kind::ResourcePacks).unwrap().into_iter().map(|i| i.name).collect();
        assert_eq!(names, vec!["faithful.zip".to_string(), "MyPack".to_string()]);
        assert!(list(&d, Kind::ShaderPacks).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn rejects_path_tricks() {
        let d = tmp("d");
        ensure_dirs(&d, Loader::Fabric).unwrap();
        for bad in ["../x.jar", "a/b.jar", "a\\b.jar", "..", "", "C:x.jar"] {
            assert!(set_mod_enabled(&d, bad, false).is_err(), "{bad}");
            assert!(delete(&d, Kind::Mods, bad).is_err(), "{bad}");
        }
        assert!(set_mod_enabled(&d, "notajar.txt", false).is_err());
        assert_eq!(Kind::parse("mods"), Some(Kind::Mods));
        assert_eq!(Kind::parse("../etc"), None);
        let _ = std::fs::remove_dir_all(&d);
    }
}
