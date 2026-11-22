//! تشخيص الشبكة: يختبر الوصول إلى كل خادم رسمي يحتاجه اللانشر ويعرض سبب الفشل الحقيقي (DNS / TLS / حجب…).
use serde::Serialize;

pub const ENDPOINTS: &[(&str, &str)] = &[
    ("Mojang version manifest", "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"),
    ("Mojang Java runtimes", "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json"),
    ("Fabric meta", "https://meta.fabricmc.net/v2/versions/loader/1.20.1"),
    ("Quilt meta", "https://meta.quiltmc.org/v3/versions/loader/1.20.1"),
    ("Forge promotions", "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json"),
    ("NeoForge maven", "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge"),
    ("Modrinth API", "https://api.modrinth.com/v2/"),
];

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub url: String,
    pub ok: bool,
    pub detail: String,
}

pub async fn run(client: &reqwest::Client) -> Vec<Check> {
    let mut out = Vec::new();
    for (name, url) in ENDPOINTS {
        let started = std::time::Instant::now();
        let (ok, detail) = match client.get(*url).send().await {
            Ok(r) if r.status().is_success() => (true, format!("HTTP {} in {} ms", r.status().as_u16(), started.elapsed().as_millis())),
            Ok(r) => (false, format!("HTTP {}", r.status())),
            Err(e) => (false, crate::Error::from(e).to_string()),
        };
        out.push(Check { name: name.to_string(), url: url.to_string(), ok, detail });
    }
    out
}
