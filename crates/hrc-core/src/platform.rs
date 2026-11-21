use std::path::PathBuf;

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

/// مجلد بيانات التطبيق (قابل للتجاوز بمتغير HRC_DATA للاختبارات).
pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("HRC_DATA") {
        return PathBuf::from(p);
    }
    directories::ProjectDirs::from("io", "horizon", "HorizonRustClean")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./hrc-data"))
}
