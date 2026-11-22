//! "Rust mods" (beta) لـ Horizon Client: ميزات صغيرة تنفَّذ بـ Rust داخل اللانشر قبل تشغيل اللعبة، وتُفعَّل/تُطفأ لكل profile.
//! لا تعدّل Minecraft ولا تحقن شيئاً فيها؛ كلها مطفأة افتراضياً وتعمل فقط على مجلد اللعبة الخاص بالـ instance.
use std::path::Path;
use std::time::{Duration, SystemTime};

pub const IDS: [&str; 3] = ["log_cleaner", "crash_cleaner", "screenshot_sorter"];

pub fn describe(id: &str) -> &'static str {
    match id {
        "log_cleaner" => "Delete compressed logs older than 14 days",
        "crash_cleaner" => "Delete crash reports older than 30 days",
        "screenshot_sorter" => "Sort screenshots into dated folders (screenshots/YYYY-MM-DD)",
        _ => "",
    }
}

const DAY: u64 = 86_400;

fn delete_old(dir: &Path, ext_ok: impl Fn(&str) -> bool, max_age: Duration, now: SystemTime) -> usize {
    let mut n = 0;
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Ok(meta) = e.metadata() else { continue };
        if !meta.is_file() || !ext_ok(&name) {
            continue;
        }
        let old = meta.modified().ok().and_then(|m| now.duration_since(m).ok()).map_or(false, |a| a > max_age);
        if old && std::fs::remove_file(e.path()).is_ok() {
            n += 1;
        }
    }
    n
}

/// اسم لقطة Minecraft: `2026-11-22_12.44.10.png` → `2026-11-22`.
pub fn screenshot_day(name: &str) -> Option<&str> {
    let d = name.get(..10)?;
    let b = d.as_bytes();
    let ok = name.ends_with(".png")
        && b[4] == b'-'
        && b[7] == b'-'
        && d.char_indices().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    ok.then_some(d)
}

fn sort_screenshots(dir: &Path) -> usize {
    let mut n = 0;
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !e.metadata().map_or(false, |m| m.is_file()) {
            continue;
        }
        if let Some(day) = screenshot_day(&name) {
            let target = dir.join(day);
            if std::fs::create_dir_all(&target).is_ok() && std::fs::rename(e.path(), target.join(&name)).is_ok() {
                n += 1;
            }
        }
    }
    n
}

/// يشغّل المفعّل فقط ويعيد سطور تقرير للـ Console.
pub fn run_prelaunch(game_dir: &Path, enabled: &std::collections::BTreeMap<String, bool>, now: SystemTime) -> Vec<String> {
    let on = |id: &str| enabled.get(id).copied().unwrap_or(false);
    let mut out = Vec::new();
    if on("log_cleaner") {
        let n = delete_old(&game_dir.join("logs"), |f| f.ends_with(".log.gz"), Duration::from_secs(14 * DAY), now);
        out.push(format!("[rust-mod] log_cleaner: removed {n} old log file(s)"));
    }
    if on("crash_cleaner") {
        let n = delete_old(&game_dir.join("crash-reports"), |f| f.ends_with(".txt"), Duration::from_secs(30 * DAY), now);
        out.push(format!("[rust-mod] crash_cleaner: removed {n} old crash report(s)"));
    }
    if on("screenshot_sorter") {
        let n = sort_screenshots(&game_dir.join("screenshots"));
        out.push(format!("[rust-mod] screenshot_sorter: moved {n} screenshot(s)"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("hrc-rm-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn screenshot_names() {
        assert_eq!(screenshot_day("2026-11-22_12.44.10.png"), Some("2026-11-22"));
        assert_eq!(screenshot_day("2026-11-22_12.44.10.jpg"), None);
        assert_eq!(screenshot_day("my pic.png"), None);
        assert_eq!(screenshot_day("x.png"), None);
    }

    #[test]
    fn disabled_does_nothing_enabled_works() {
        let d = tmp("a");
        std::fs::create_dir_all(d.join("logs")).unwrap();
        std::fs::create_dir_all(d.join("crash-reports")).unwrap();
        std::fs::create_dir_all(d.join("screenshots")).unwrap();
        std::fs::write(d.join("logs/old.log.gz"), b"x").unwrap();
        std::fs::write(d.join("logs/latest.log"), b"x").unwrap();
        std::fs::write(d.join("crash-reports/crash-1.txt"), b"x").unwrap();
        std::fs::write(d.join("screenshots/2026-11-22_12.44.10.png"), b"x").unwrap();
        let future = SystemTime::now() + Duration::from_secs(100 * DAY); // يجعل الملفات "قديمة"
        assert!(run_prelaunch(&d, &BTreeMap::new(), future).is_empty());
        assert!(d.join("logs/old.log.gz").exists() && d.join("crash-reports/crash-1.txt").exists());

        let mut on = BTreeMap::new();
        for id in IDS {
            on.insert(id.to_string(), true);
        }
        let rep = run_prelaunch(&d, &on, future);
        assert_eq!(rep.len(), 3);
        assert!(!d.join("logs/old.log.gz").exists());
        assert!(d.join("logs/latest.log").exists(), "latest.log must never be deleted");
        assert!(!d.join("crash-reports/crash-1.txt").exists());
        assert!(d.join("screenshots/2026-11-22/2026-11-22_12.44.10.png").is_file());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn recent_files_are_kept() {
        let d = tmp("b");
        std::fs::create_dir_all(d.join("logs")).unwrap();
        std::fs::write(d.join("logs/new.log.gz"), b"x").unwrap();
        let mut on = BTreeMap::new();
        on.insert("log_cleaner".to_string(), true);
        run_prelaunch(&d, &on, SystemTime::now());
        assert!(d.join("logs/new.log.gz").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
