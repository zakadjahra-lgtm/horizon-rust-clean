//! تفضيلات مظهر بسيطة تُضاف لـ `options.txt` الخاص بالـ instance عند أول تشغيل: خلفية شاشة بداية
//! Mojang سوداء (خيار اللعبة نفسه `darkMojangStudiosBackground`، موجود منذ 1.16). لا نغيّر ملفات ماينكرافت،
//! ولا نكتب فوق اختيار اللاعب: إن وُجد المفتاح في الملف (true أو false) نتركه كما هو.
use std::path::Path;

const KEY: &str = "darkMojangStudiosBackground";

/// هل الإصدار يعرف هذا الخيار؟ إصدارات 1.16 فما فوق وسلسلة 26.x. Snapshots وغيرها: لا.
pub fn supports_dark_splash(version_id: &str) -> bool {
    if let Some(rest) = version_id.strip_prefix("1.") {
        let minor: u32 = rest.split(|c: char| !c.is_ascii_digit()).next().and_then(|m| m.parse().ok()).unwrap_or(0);
        return minor >= 16;
    }
    version_id.split('.').next().and_then(|m| m.parse::<u32>().ok()).is_some_and(|major| (20..100).contains(&major))
        && version_id.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// يضيف `darkMojangStudiosBackground:true` إن لم يكن المفتاح موجوداً. يعيد true إذا كتب.
pub fn ensure_dark_splash(game_dir: &Path, version_id: &str) -> std::io::Result<bool> {
    if !supports_dark_splash(version_id) {
        return Ok(false);
    }
    let file = game_dir.join("options.txt");
    let mut text = std::fs::read_to_string(&file).unwrap_or_default();
    if text.lines().any(|l| l.split(':').next() == Some(KEY)) {
        return Ok(false);
    }
    std::fs::create_dir_all(game_dir)?;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("{KEY}:true\n"));
    std::fs::write(&file, text)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_versions_that_have_the_option() {
        for v in ["1.16", "1.16.5", "1.20.1", "1.21.1", "26.3"] {
            assert!(supports_dark_splash(v), "{v}");
        }
        for v in ["1.8.9", "1.12.2", "1.15.2", "20w14a", "abc", ""] {
            assert!(!supports_dark_splash(v), "{v}");
        }
    }

    #[test]
    fn adds_once_and_never_overrides_the_players_choice() {
        let d = std::env::temp_dir().join(format!("hrc-brand-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        assert!(ensure_dark_splash(&d, "1.20.1").unwrap());
        assert!(std::fs::read_to_string(d.join("options.txt")).unwrap().contains("darkMojangStudiosBackground:true"));
        assert!(!ensure_dark_splash(&d, "1.20.1").unwrap());
        std::fs::write(d.join("options.txt"), "fov:70.0\ndarkMojangStudiosBackground:false").unwrap();
        assert!(!ensure_dark_splash(&d, "1.20.1").unwrap());
        assert!(std::fs::read_to_string(d.join("options.txt")).unwrap().contains("darkMojangStudiosBackground:false"));
        assert!(!ensure_dark_splash(&d, "1.8.9").unwrap());
        let _ = std::fs::remove_dir_all(&d);
    }
}
