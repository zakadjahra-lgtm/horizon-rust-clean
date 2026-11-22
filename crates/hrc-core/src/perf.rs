//! تحسينات الأداء: presets لإعدادات JVM + أهلية مودات Sodium/Lithium حسب إصدار Minecraft.
//! ملاحظة صادقة: هذه الإعدادات قياسية ومعروفة، لكن مكاسبها لم تُقَس في مشروعنا بعد.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PerfPreset {
    /// Vanilla Clean: لا أي وسيط JVM إضافي.
    #[default]
    Vanilla,
    /// G1GC بإعدادات معتدلة لتقليل التقطّع.
    Balanced,
    /// Balanced + إزالة تكرار النصوص وهدف توقف أقصر.
    Performance,
}

impl PerfPreset {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "vanilla" => Some(Self::Vanilla),
            "balanced" => Some(Self::Balanced),
            "performance" => Some(Self::Performance),
            _ => None,
        }
    }
}

/// وسائط JVM صالحة على Java 8 وحتى 21 (لا أعلام محذوفة مثل CMS).
pub fn jvm_flags(preset: PerfPreset) -> Vec<String> {
    let base = [
        "-XX:+UseG1GC",
        "-XX:+ParallelRefProcEnabled",
        "-XX:MaxGCPauseMillis=50",
        "-XX:+UnlockExperimentalVMOptions", // مطلوب لأعلام G1 التجريبية أدناه
        "-XX:G1NewSizePercent=20",
        "-XX:G1ReservePercent=20",
        "-XX:+DisableExplicitGC",
    ];
    let mut v: Vec<String> = match preset {
        PerfPreset::Vanilla => return Vec::new(),
        _ => base.iter().map(|s| s.to_string()).collect(),
    };
    if preset == PerfPreset::Performance {
        v.retain(|f| !f.starts_with("-XX:MaxGCPauseMillis"));
        v.push("-XX:MaxGCPauseMillis=30".into());
        v.push("-XX:+UseStringDeduplication".into());
    }
    v
}

/// يحلل معرّف إصدار release مثل `1.8.9` / `1.21` / `26.1` إلى (major, minor, patch).
/// أي شيء آخر (snapshot، beta، pre-release…) يعيد None.
pub fn parse_mc_version(id: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = id.split('.').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let n: Option<Vec<u32>> = parts.iter().map(|p| p.parse().ok()).collect();
    let n = n?;
    Some((n[0], n[1], n.get(2).copied().unwrap_or(0)))
}

/// Sodium: Fabric/NeoForge، من 1.16.3 (حسب صفحة Modrinth الرسمية). لا يدعم 1.8–1.12.2.
pub fn sodium_supported(id: &str) -> bool {
    match parse_mc_version(id) {
        Some((1, m, p)) => m >= 17 || (m == 16 && p >= 3),
        Some((major, _, _)) => major >= 26, // نظام الترقيم الجديد
        None => false,
    }
}

/// Lithium: من 1.16.2.
pub fn lithium_supported(id: &str) -> bool {
    match parse_mc_version(id) {
        Some((1, m, p)) => m >= 17 || (m == 16 && p >= 2),
        Some((major, _, _)) => major >= 26,
        None => false,
    }
}

/// طبقة HUD في `client/` تعمل على LWJGL 2 فقط: 1.8 – 1.12.2.
pub fn legacy_hud_supported(id: &str) -> bool {
    matches!(parse_mc_version(id), Some((1, m, _)) if (8..=12).contains(&m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_adds_nothing() {
        assert!(jvm_flags(PerfPreset::Vanilla).is_empty());
    }

    #[test]
    fn balanced_and_performance_flags() {
        let b = jvm_flags(PerfPreset::Balanced);
        assert!(b.contains(&"-XX:+UseG1GC".to_string()));
        // الأعلام التجريبية يجب أن تسبقها Unlock
        let unlock = b.iter().position(|f| f == "-XX:+UnlockExperimentalVMOptions").unwrap();
        let g1 = b.iter().position(|f| f == "-XX:G1NewSizePercent=20").unwrap();
        assert!(unlock < g1);
        let p = jvm_flags(PerfPreset::Performance);
        assert_eq!(p.iter().filter(|f| f.starts_with("-XX:MaxGCPauseMillis")).count(), 1);
        assert!(p.contains(&"-XX:MaxGCPauseMillis=30".to_string()));
        assert!(p.contains(&"-XX:+UseStringDeduplication".to_string()));
    }

    #[test]
    fn preset_parse() {
        assert_eq!(PerfPreset::parse(" Balanced "), Some(PerfPreset::Balanced));
        assert_eq!(PerfPreset::parse("x"), None);
    }

    #[test]
    fn version_parsing() {
        assert_eq!(parse_mc_version("1.8.9"), Some((1, 8, 9)));
        assert_eq!(parse_mc_version("1.21"), Some((1, 21, 0)));
        assert_eq!(parse_mc_version("26.1"), Some((26, 1, 0)));
        assert_eq!(parse_mc_version("1.14.4-pre1"), None);
        assert_eq!(parse_mc_version("b1.7.3"), None);
        assert_eq!(parse_mc_version("24w01a"), None);
    }

    #[test]
    fn mod_eligibility_matrix() {
        for v in ["1.8.9", "1.12.2", "1.16.2", "b1.7.3", "24w01a"] {
            assert!(!sodium_supported(v), "sodium {v}");
        }
        assert!(!sodium_supported("1.16.2") && sodium_supported("1.16.3") && sodium_supported("1.16.5"));
        assert!(!lithium_supported("1.16.1") && lithium_supported("1.16.2"));
        assert!(sodium_supported("1.20.1") && lithium_supported("1.21") && sodium_supported("26.1"));
    }

    #[test]
    fn legacy_hud_range() {
        assert!(legacy_hud_supported("1.8") && legacy_hud_supported("1.8.9") && legacy_hud_supported("1.12.2"));
        assert!(!legacy_hud_supported("1.7.10") && !legacy_hud_supported("1.13") && !legacy_hud_supported("1.20.1"));
    }
}
