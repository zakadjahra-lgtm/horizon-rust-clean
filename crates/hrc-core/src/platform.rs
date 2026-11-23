use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    Osx,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Osx
        } else {
            Os::Linux
        }
    }
    /// الاسم المستخدم في Version JSON (`windows` / `linux` / `osx`).
    pub fn json_name(self) -> &'static str {
        match self {
            Os::Windows => "windows",
            Os::Linux => "linux",
            Os::Osx => "osx",
        }
    }
    pub fn classpath_separator(self) -> &'static str {
        if self == Os::Windows { ";" } else { ":" }
    }
    pub fn java_exe(self) -> &'static str {
        if self == Os::Windows { "javaw.exe" } else { "java" }
    }
}

/// `x86` أو `x86_64` أو `arm64` كما تستخدمها قواعد Mojang.
pub fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x86_64",
    }
}

/// مفتاح المنصة في all.json الخاص بـ Java Runtime الرسمي.
pub fn java_platform_key() -> &'static str {
    match (Os::current(), std::env::consts::ARCH) {
        (Os::Windows, "x86") => "windows-x86",
        (Os::Windows, "aarch64") => "windows-arm64",
        (Os::Windows, _) => "windows-x64",
        (Os::Osx, "aarch64") => "mac-os-arm64",
        (Os::Osx, _) => "mac-os",
        (Os::Linux, "x86") => "linux-i386",
        (Os::Linux, _) => "linux",
    }
}

/// مجلد بيانات التطبيق (قابل للتجاوز بمتغير HRC_DATA للاختبارات): `MClauncher` داخل مجلد بيانات
/// المستخدم (على Windows: `%APPDATA%\\MClauncher`). إن وُجد مجلد الاسم القديم ولم يوجد الجديد يُنقل إليه.
pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("HRC_DATA") {
        return PathBuf::from(p);
    }
    let Some(base) = directories::BaseDirs::new() else { return PathBuf::from("./hrc-data") };
    let new = base.data_dir().join(DATA_FOLDER);
    let old = directories::ProjectDirs::from("io", "horizon", "HorizonRustClean").map(|d| d.data_dir().to_path_buf());
    if let Some(old) = old {
        let _ = migrate(&old, &new);
    }
    new
}

pub const DATA_FOLDER: &str = "MClauncher";

/// ينقل مجلد البيانات القديم إلى الجديد مرة واحدة: فقط إن وُجد القديم ولم يوجد الجديد (لا يستبدل شيئاً).
pub fn migrate(old: &Path, new: &Path) -> std::io::Result<bool> {
    if !old.is_dir() || new.exists() {
        return Ok(false);
    }
    if let Some(parent) = new.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(old, new)?;
    Ok(true)
}

#[cfg(test)]
mod migrate_tests {
    use super::*;

    #[test]
    fn old_data_moves_once_and_never_overwrites() {
        let root = std::env::temp_dir().join(format!("hrc-mig-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (old, new) = (root.join("old"), root.join("MClauncher"));
        std::fs::create_dir_all(old.join("instances")).unwrap();
        std::fs::write(old.join("settings.json"), "{}").unwrap();
        assert!(migrate(&old, &new).unwrap());
        assert!(new.join("settings.json").is_file() && !old.exists());
        // الجديد موجود الآن: لا نقل ولا استبدال حتى لو عاد القديم.
        std::fs::create_dir_all(&old).unwrap();
        assert!(!migrate(&old, &new).unwrap());
        assert!(old.is_dir() && new.join("settings.json").is_file());
        // لا قديم: لا شيء.
        assert!(!migrate(&root.join("none"), &root.join("other")).unwrap());
        let _ = std::fs::remove_dir_all(&root);
    }
}
