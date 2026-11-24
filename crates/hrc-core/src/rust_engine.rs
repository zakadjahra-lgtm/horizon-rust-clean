//! محرك Rust (لعبة Rust الأصلية) كنظام مستقل: له مجلده `rust-engine/` وإعداداته وملفات عالمه،
//! لا علاقة له بـ instances ماينكرافت جافا ولا بالكلاينتات الخارجية.
//! الإعدادات تُكتب في `horizon-rust.properties` بالصيغة التي تقرؤها اللعبة نفسها (`hrc-rustgame`).
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Props {
    pub seed: u32,
    pub radius: i32,
    pub vsync: bool,
}

impl Default for Props {
    fn default() -> Self {
        Props { seed: 1337, radius: 8, vsync: false }
    }
}

pub const FILE: &str = "horizon-rust.properties";
/// نفس مدى `radius` الذي تقبله اللعبة.
pub const RADIUS: (i32, i32) = (1, 16);

pub fn dir(data: &Path) -> PathBuf {
    data.join("rust-engine").join("game")
}

impl Props {
    pub fn clamped(mut self) -> Props {
        self.radius = self.radius.clamp(RADIUS.0, RADIUS.1);
        self
    }
    pub fn parse(text: &str) -> Props {
        let mut p = Props::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match (k.trim(), v.trim()) {
                ("seed", v) => p.seed = v.parse().unwrap_or(p.seed),
                ("radius", v) => p.radius = v.parse().unwrap_or(p.radius),
                ("vsync", v) => p.vsync = v.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
        p.clamped()
    }
    pub fn render(&self) -> String {
        let p = self.clone().clamped();
        format!("seed={}\nradius={}\nvsync={}\n", p.seed, p.radius, p.vsync)
    }
}

pub fn load(data: &Path) -> Props {
    std::fs::read_to_string(dir(data).join(FILE)).map(|t| Props::parse(&t)).unwrap_or_default()
}

pub fn save(data: &Path, p: &Props) -> std::io::Result<()> {
    let d = dir(data);
    std::fs::create_dir_all(&d)?;
    std::fs::write(d.join(FILE), p.render())
}

/// هل للمحرك عالم محفوظ (موضع اللاعب)؟
pub fn has_world(data: &Path) -> bool {
    dir(data).join("horizon-rust-save.txt").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_round_trip_and_clamp() {
        let p = Props { seed: 42, radius: 12, vsync: true };
        assert_eq!(Props::parse(&p.render()), p);
        assert_eq!(Props::parse("radius=99").radius, 16);
        assert_eq!(Props::parse("radius=-5").radius, 1);
        assert_eq!(Props::parse("garbage\nseed=x"), Props::default());
    }

    #[test]
    fn saving_uses_the_engine_folder_only() {
        let root = std::env::temp_dir().join(format!("hrc-engine-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(load(&root), Props::default());
        save(&root, &Props { seed: 7, radius: 3, vsync: false }).unwrap();
        assert_eq!(load(&root).seed, 7);
        assert!(root.join("rust-engine/game").join(FILE).is_file());
        assert!(!root.join("instances").exists());
        assert!(!has_world(&root));
        let _ = std::fs::remove_dir_all(&root);
    }
}
