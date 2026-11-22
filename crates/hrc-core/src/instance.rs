//! Instances: كل instance له مجلد مستقل (لعبة + natives + إعدادات)، فلا اختلاط بين الإصدارات.
//! الملفات المشتركة (libraries/assets/versions) تبقى في `shared/` ولا تُحذف مع الـ instance.
use crate::{err, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub max_memory_mb: u32,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub created_unix: u64,
    /// Profile العميل (HUD + أداء) المرتبط بهذا الـ instance؛ None = بلا تعديلات.
    #[serde(default)]
    pub profile_id: Option<String>,
}

pub fn instances_dir(data: &Path) -> PathBuf {
    data.join("instances")
}

/// المعرّف يأتي من الواجهة، لذا يُقبل فقط بصيغة UUID (حماية من path traversal).
pub(crate) fn valid_id(id: &str) -> bool {
    id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

pub fn root(data: &Path, id: &str) -> PathBuf {
    instances_dir(data).join(id)
}
pub fn game_dir(data: &Path, id: &str) -> PathBuf {
    root(data, id).join("game")
}
pub fn natives_dir(data: &Path, id: &str) -> PathBuf {
    root(data, id).join("natives")
}
fn json_path(data: &Path, id: &str) -> PathBuf {
    root(data, id).join("instance.json")
}

fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 64 {
        return err("اسم الـ instance يجب أن يكون بين 1 و64 حرفاً");
    }
    Ok(n.to_string())
}

pub fn load(data: &Path, id: &str) -> Result<Instance> {
    if !valid_id(id) {
        return err("معرّف instance غير صالح");
    }
    Ok(serde_json::from_slice(&std::fs::read(json_path(data, id))?)?)
}

pub fn save(data: &Path, inst: &Instance) -> Result<()> {
    if !valid_id(&inst.id) {
        return err("معرّف instance غير صالح");
    }
    std::fs::create_dir_all(root(data, &inst.id))?;
    std::fs::write(json_path(data, &inst.id), serde_json::to_vec_pretty(inst)?)?;
    Ok(())
}

pub fn list(data: &Path) -> Result<Vec<Instance>> {
    let mut out = Vec::new();
    let dir = instances_dir(data);
    if !dir.exists() {
        return Ok(out);
    }
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let id = e.file_name().to_string_lossy().into_owned();
        if let Ok(i) = load(data, &id) {
            out.push(i);
        }
    }
    out.sort_by_key(|i| i.created_unix);
    Ok(out)
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn create(data: &Path, name: &str, version_id: &str, max_memory_mb: u32) -> Result<Instance> {
    if version_id.trim().is_empty() {
        return err("اختر إصداراً");
    }
    let inst = Instance {
        id: uuid::Uuid::new_v4().to_string(),
        name: clean_name(name)?,
        version_id: version_id.to_string(),
        max_memory_mb: max_memory_mb.clamp(512, 65536),
        java_path: None,
        installed: false,
        created_unix: now_unix(),
        profile_id: None,
    };
    std::fs::create_dir_all(game_dir(data, &inst.id))?;
    save(data, &inst)?;
    Ok(inst)
}

pub fn rename(data: &Path, id: &str, name: &str) -> Result<()> {
    let mut i = load(data, id)?;
    i.name = clean_name(name)?;
    save(data, &i)
}

pub fn set_memory(data: &Path, id: &str, mb: u32) -> Result<()> {
    let mut i = load(data, id)?;
    i.max_memory_mb = mb.clamp(512, 65536);
    save(data, &i)
}

pub fn set_profile(data: &Path, id: &str, profile_id: Option<String>) -> Result<()> {
    if let Some(p) = &profile_id {
        if !valid_id(p) {
            return err("معرّف profile غير صالح");
        }
    }
    let mut i = load(data, id)?;
    i.profile_id = profile_id;
    save(data, &i)
}

pub fn set_installed(data: &Path, id: &str, installed: bool) -> Result<()> {
    let mut i = load(data, id)?;
    i.installed = installed;
    save(data, &i)
}

/// يحذف مجلد الـ instance فقط (عوالمه وإعداداته وnatives). ملفات `shared/` لا تُمس.
pub fn delete(data: &Path, id: &str) -> Result<()> {
    if !valid_id(id) {
        return err("معرّف instance غير صالح");
    }
    let r = root(data, id);
    if r.exists() {
        std::fs::remove_dir_all(r)?;
    }
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let (from, to) = (e.path(), dst.join(e.file_name()));
        if e.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// نسخة كاملة (العوالم والإعدادات والـ natives) باسم جديد ومعرّف جديد.
pub fn duplicate(data: &Path, id: &str) -> Result<Instance> {
    let src = load(data, id)?;
    let mut new = src.clone();
    new.id = uuid::Uuid::new_v4().to_string();
    new.name = format!("{} (copy)", src.name.chars().take(55).collect::<String>());
    new.created_unix = now_unix();
    let dst = root(data, &new.id);
    copy_dir(&root(data, id), &dst)?;
    save(data, &new)?; // يستبدل instance.json المنسوخ
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hrc-inst-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn create_list_rename_memory_delete() {
        let d = tmp("a");
        let a = create(&d, "  Survival  ", "1.8.9", 2048).unwrap();
        let b = create(&d, "PvP", "1.8.9", 100).unwrap(); // أقل من الحد الأدنى
        assert_eq!(a.name, "Survival");
        assert_eq!(b.max_memory_mb, 512);
        assert_eq!(list(&d).unwrap().len(), 2);
        rename(&d, &a.id, "Renamed").unwrap();
        set_memory(&d, &a.id, 4096).unwrap();
        let a2 = load(&d, &a.id).unwrap();
        assert_eq!((a2.name.as_str(), a2.max_memory_mb), ("Renamed", 4096));
        delete(&d, &a.id).unwrap();
        assert_eq!(list(&d).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn same_version_twice_gets_separate_folders() {
        let d = tmp("b");
        let a = create(&d, "One", "1.12.2", 2048).unwrap();
        let b = create(&d, "Two", "1.12.2", 2048).unwrap();
        assert_ne!(game_dir(&d, &a.id), game_dir(&d, &b.id));
        std::fs::write(game_dir(&d, &a.id).join("options.txt"), "x").unwrap();
        assert!(!game_dir(&d, &b.id).join("options.txt").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn duplicate_copies_files_with_new_id() {
        let d = tmp("c");
        let a = create(&d, "World", "1.16.5", 2048).unwrap();
        std::fs::write(game_dir(&d, &a.id).join("options.txt"), "fov:90").unwrap();
        let b = duplicate(&d, &a.id).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(std::fs::read_to_string(game_dir(&d, &b.id).join("options.txt")).unwrap(), "fov:90");
        assert!(b.name.ends_with("(copy)"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn profile_link_is_per_instance_and_validated() {
        let d = tmp("e");
        let a = create(&d, "A", "1.8.9", 2048).unwrap();
        let b = create(&d, "B", "1.20.1", 2048).unwrap();
        let pid = "00000000-0000-0000-0000-000000000009".to_string();
        set_profile(&d, &a.id, Some(pid.clone())).unwrap();
        assert_eq!(load(&d, &a.id).unwrap().profile_id, Some(pid));
        assert_eq!(load(&d, &b.id).unwrap().profile_id, None);
        assert!(set_profile(&d, &a.id, Some("../x".into())).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn rejects_path_traversal_ids_and_empty_names() {
        let d = tmp("d");
        assert!(load(&d, "../../etc").is_err());
        assert!(delete(&d, "..").is_err());
        assert!(create(&d, "   ", "1.8.9", 2048).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
