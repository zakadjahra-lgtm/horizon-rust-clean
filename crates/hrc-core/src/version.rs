//! نماذج Version JSON (الصيغتان القديمة والحديثة) ودمج `inheritsFrom`.
use crate::rules::{allowed, Features, Rule};
use crate::platform::Os;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub main_class: Option<String>,
    /// الصيغة القديمة (≤ 1.12.2): نص واحد مفصول بمسافات.
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    /// الصيغة الحديثة (≥ 1.13).
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<Downloads>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
    #[serde(default)]
    pub logging: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Arg>,
    #[serde(default)]
    pub jvm: Vec<Arg>,
}

/// وسيط: نص بسيط أو كائن له rules وقيمة (نص أو قائمة).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Arg {
    Plain(String),
    Ruled { #[serde(default)] rules: Vec<Rule>, value: ArgValue },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// أسماء classifier للـ natives: {"linux":"natives-linux","windows":"natives-windows"}
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
    /// مكتبات Fabric/Forge القديمة بلا downloads تستخدم url + مسار Maven.
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub total_size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    pub client: Option<Artifact>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u32,
}

impl VersionJson {
    pub fn parse(s: &str) -> crate::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    /// دمج الإصدار الابن فوق الأب (Fabric/Forge يستخدمان inheritsFrom).
    pub fn merged_with_parent(mut self, parent: VersionJson) -> VersionJson {
        let mut libs = self.libraries.clone();
        libs.extend(parent.libraries.iter().cloned());
        self.libraries = libs;

        self.main_class = self.main_class.or(parent.main_class);
        self.asset_index = self.asset_index.or(parent.asset_index);
        self.assets = self.assets.or(parent.assets);
        self.downloads = self.downloads.or(parent.downloads);
        self.java_version = self.java_version.or(parent.java_version);
        self.minecraft_arguments = self.minecraft_arguments.or(parent.minecraft_arguments);
        self.logging = self.logging.or(parent.logging);

        self.arguments = match (self.arguments.take(), parent.arguments) {
            (Some(mut c), Some(p)) => {
                let mut game = p.game;
                game.extend(c.game.drain(..));
                let mut jvm = p.jvm;
                jvm.extend(c.jvm.drain(..));
                Some(Arguments { game, jvm })
            }
            (c, p) => c.or(p),
        };
        self
    }

    /// إصدار Java المطلوب؛ الإصدارات القديمة بلا الحقل تعمل على Java 8.
    pub fn required_java_major(&self) -> u32 {
        self.java_version.as_ref().map(|j| j.major_version).unwrap_or(8)
    }

    pub fn java_component(&self) -> &str {
        self.java_version
            .as_ref()
            .map(|j| j.component.as_str())
            .unwrap_or("jre-legacy")
    }

    /// الوسائط بعد التقييم: (jvm, game). للصيغة القديمة تُبنى jvm الافتراضية.
    pub fn resolved_arguments(&self, os: Os, f: &Features) -> (Vec<String>, Vec<String>) {
        if let Some(a) = &self.arguments {
            (flatten(&a.jvm, os, f), flatten(&a.game, os, f))
        } else {
            let game = self
                .minecraft_arguments
                .as_deref()
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_string)
                .collect();
            (legacy_jvm_args(os), game)
        }
    }
}

fn flatten(args: &[Arg], os: Os, f: &Features) -> Vec<String> {
    let mut out = Vec::new();
    for a in args {
        match a {
            Arg::Plain(s) => out.push(s.clone()),
            Arg::Ruled { rules, value } => {
                if allowed(rules, os, f) {
                    match value {
                        ArgValue::One(s) => out.push(s.clone()),
                        ArgValue::Many(v) => out.extend(v.iter().cloned()),
                    }
                }
            }
        }
    }
    out
}

/// JVM الافتراضية للإصدارات القديمة التي لا تحمل `arguments.jvm`.
pub fn legacy_jvm_args(os: Os) -> Vec<String> {
    let mut v = vec![
        "-Djava.library.path=${natives_directory}".to_string(),
        "-cp".to_string(),
        "${classpath}".to_string(),
    ];
    if os == Os::Windows {
        v.insert(0, "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump".into());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = r#"{
      "id":"1.8.9","type":"release","mainClass":"net.minecraft.client.main.Main",
      "minecraftArguments":"--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory}",
      "assets":"1.8","libraries":[{"name":"a:b:1"}]
    }"#;

    const NEW: &str = r#"{
      "id":"1.20.1","mainClass":"net.minecraft.client.main.Main",
      "arguments":{
        "game":["--username","${auth_player_name}",
                {"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"}],
        "jvm":[{"rules":[{"action":"allow","os":{"name":"osx"}}],"value":["-XstartOnFirstThread"]},
               "-Djava.library.path=${natives_directory}","-cp","${classpath}"]
      },
      "javaVersion":{"component":"java-runtime-gamma","majorVersion":17},
      "libraries":[]
    }"#;

    #[test]
    fn legacy_arguments_are_split_and_get_default_jvm() {
        let v = VersionJson::parse(OLD).unwrap();
        let (jvm, game) = v.resolved_arguments(Os::Linux, &Features::default());
        assert_eq!(game[0], "--username");
        assert!(jvm.contains(&"${classpath}".to_string()));
        assert_eq!(v.required_java_major(), 8);
    }

    #[test]
    fn modern_arguments_respect_rules() {
        let v = VersionJson::parse(NEW).unwrap();
        let (jvm, game) = v.resolved_arguments(Os::Linux, &Features::default());
        assert!(!jvm.contains(&"-XstartOnFirstThread".to_string()));
        assert!(!game.contains(&"--demo".to_string()));
        let (jvm_mac, _) = v.resolved_arguments(Os::Osx, &Features::default());
        assert!(jvm_mac.contains(&"-XstartOnFirstThread".to_string()));
        assert_eq!(v.required_java_major(), 17);
    }
}
