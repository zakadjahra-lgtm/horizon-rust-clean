//! Horizon Rust Clean — المحرك الأساسي. لا يحتوي أي منطق للعبة؛ يدير ملفات Mojang
//! الرسمية ويشغّل Minecraft الأصلية عبر Java.

pub mod content;
pub mod download;
pub mod install;
pub mod instance;
pub mod java;
pub mod launch;
pub mod loader;
pub mod manifest;
pub mod modcheck;
pub mod modrinth;
pub mod perf;
pub mod platform;
pub mod profile;
pub mod rules;
pub mod rustmods;
pub mod settings;
pub mod version;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Http(String),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("SHA1 mismatch for {path}: expected {expected}, got {actual}")]
    Sha1 { path: String, expected: String, actual: String },
    #[error("{0}")]
    Msg(String),
}

impl From<reqwest::Error> for Error {
    /// يعرض سلسلة الأسباب الكاملة (DNS / TLS / رفض اتصال…) بدل "error sending request" المبهمة.
    fn from(e: reqwest::Error) -> Self {
        use std::error::Error as _;
        let mut msg = e.to_string();
        let mut src = e.source();
        while let Some(s) = src {
            msg.push_str(&format!(" → {s}"));
            src = s.source();
        }
        if e.is_connect() || e.is_timeout() || msg.contains("error sending request") {
            msg.push_str(" | Check your internet, firewall/antivirus (HTTPS scanning), VPN or DNS, then retry.");
        }
        Error::Http(msg)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn err<T>(m: impl Into<String>) -> Result<T> {
    Err(Error::Msg(m.into()))
}
