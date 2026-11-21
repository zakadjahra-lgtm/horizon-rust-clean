//! Minecraft Version Manifest الرسمي (Mojang).
use crate::Result;
use serde::Deserialize;

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestEntry {
    pub id: String,
    /// release | snapshot | old_beta | old_alpha
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

impl Manifest {
    pub async fn fetch(client: &reqwest::Client) -> Result<Manifest> {
        Ok(client.get(MANIFEST_URL).send().await?.error_for_status()?.json().await?)
    }

    pub fn find(&self, id: &str) -> Option<&ManifestEntry> {
        self.versions.iter().find(|v| v.id == id)
    }

    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a ManifestEntry> {
        self.versions.iter().filter(move |v| v.kind == kind)
    }

    pub fn latest_release(&self) -> Option<&ManifestEntry> {
        self.find(&self.latest.release)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_shape() {
        let m: Manifest = serde_json::from_str(
            r#"{"latest":{"release":"1.21","snapshot":"24w01a"},
                "versions":[{"id":"1.21","type":"release","url":"u","sha1":"s"},
                            {"id":"b1.7.3","type":"old_beta","url":"u2"}]}"#,
        )
        .unwrap();
        assert_eq!(m.latest_release().unwrap().id, "1.21");
        assert_eq!(m.of_kind("old_beta").count(), 1);
    }
}
