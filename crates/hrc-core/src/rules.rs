//! تقييم `rules` في Version JSON (للمكتبات والـ arguments).
use crate::platform::{arch_name, Os};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: String, // "allow" | "disallow"
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

/// الميزات المفعّلة وقت التشغيل (demo, custom resolution …).
#[derive(Debug, Clone, Default)]
pub struct Features {
    pub enabled: HashMap<String, bool>,
}

impl Features {
    pub fn with(mut self, k: &str, v: bool) -> Self {
        self.enabled.insert(k.to_string(), v);
        self
    }
}

fn os_matches(r: &OsRule, os: Os) -> bool {
    if let Some(n) = &r.name {
        if n != os.json_name() {
            return false;
        }
    }
    if let Some(a) = &r.arch {
        if a != arch_name() {
            return false;
        }
    }
    // `version` (regex على إصدار النظام) تُستخدم فقط لاستثناء macOS 10.5.x القديم،
    // ولا تؤثر على الأنظمة الحديثة؛ نتجاهلها عمداً.
    true
}

fn rule_matches(r: &Rule, os: Os, f: &Features) -> bool {
    if let Some(o) = &r.os {
        if !os_matches(o, os) {
            return false;
        }
    }
    if let Some(feats) = &r.features {
        for (k, want) in feats {
            if f.enabled.get(k).copied().unwrap_or(false) != *want {
                return false;
            }
        }
    }
    true
}

/// بدون قواعد = مسموح. وإلا فآخر قاعدة مطابقة تحسم (allow/disallow)، والافتراضي disallow.
pub fn allowed(rules: &[Rule], os: Os, f: &Features) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut result = false;
    for r in rules {
        if rule_matches(r, os, f) {
            result = r.action == "allow";
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn empty_is_allowed() {
        assert!(allowed(&[], Os::Linux, &Features::default()));
    }

    #[test]
    fn allow_all_then_disallow_osx() {
        let r = rules(r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]"#);
        assert!(allowed(&r, Os::Linux, &Features::default()));
        assert!(!allowed(&r, Os::Osx, &Features::default()));
    }

    #[test]
    fn only_windows() {
        let r = rules(r#"[{"action":"allow","os":{"name":"windows"}}]"#);
        assert!(allowed(&r, Os::Windows, &Features::default()));
        assert!(!allowed(&r, Os::Linux, &Features::default()));
    }

    #[test]
    fn feature_rule_needs_flag() {
        let r = rules(r#"[{"action":"allow","features":{"is_demo_user":true}}]"#);
        assert!(!allowed(&r, Os::Linux, &Features::default()));
        assert!(allowed(&r, Os::Linux, &Features::default().with("is_demo_user", true)));
    }
}
