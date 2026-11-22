//! Horizon Rust Clean — المحرك الأساسي. لا يحتوي أي منطق للعبة؛ يدير ملفات Mojang
//! الرسمية ويشغّل Minecraft الأصلية عبر Java.

pub mod download;
pub mod install;
pub mod instance;
pub mod java;
pub mod launch;
pub mod manifest;
pub mod perf;
pub mod platform;
pub mod profile;
pub mod rules;
pub mod settings;
pub mod version;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("SHA1 mismatch for {path}: expected {expected}, got {actual}")]
    Sha1 { path: String, expected: String, actual: String },
    #[error("{0}")]
    Msg(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn err<T>(m: impl Into<String>) -> Result<T> {
    Err(Error::Msg(m.into()))
}
