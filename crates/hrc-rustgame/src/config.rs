//! إعدادات وحفظ اللعبة داخل مجلد الـ instance (`--dir`). كل instance له ملفاته المستقلة.
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub seed: u32,
    pub radius: i32,
    pub vsync: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { seed: 1337, radius: 8, vsync: false }
    }
}

impl Config {
    /// ملف `horizon-rust.properties`: أسطر key=value، والأخطاء تُتجاهل (تبقى القيم الافتراضية).
    pub fn from_properties(text: &str) -> Config {
        let mut c = Config::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match (k.trim(), v.trim()) {
                ("seed", v) => c.seed = v.parse().unwrap_or(c.seed),
                ("radius", v) => c.radius = v.parse::<i32>().map(|r| r.clamp(1, 16)).unwrap_or(c.radius),
                ("vsync", v) => c.vsync = v.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
        c
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaveState {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

pub fn format_state(s: &SaveState) -> String {
    format!("{} {} {} {} {}\n", s.pos[0], s.pos[1], s.pos[2], s.yaw, s.pitch)
}

pub fn parse_state(text: &str) -> Option<SaveState> {
    let v: Vec<f32> = text.split_whitespace().map(|x| x.parse().ok()).collect::<Option<_>>()?;
    if v.len() != 5 || v.iter().any(|x| !x.is_finite()) {
        return None;
    }
    Some(SaveState { pos: [v[0], v[1], v[2]], yaw: v[3], pitch: v[4].clamp(-1.55, 1.55) })
}

#[derive(Debug, Default, PartialEq)]
pub struct Args {
    pub dir: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub size: Option<(u32, u32)>,
    pub seed: Option<u32>,
    pub radius: Option<i32>,
    pub vsync: bool,
    pub user: Option<String>,
}

impl Args {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
        let mut a = Args::default();
        let mut it = args.into_iter();
        while let Some(k) = it.next() {
            let mut val = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
            match k.as_str() {
                "--dir" => a.dir = Some(PathBuf::from(val("--dir")?)),
                "--screenshot" => a.screenshot = Some(PathBuf::from(val("--screenshot")?)),
                "--size" => {
                    let v = val("--size")?;
                    let (w, h) = v.split_once('x').ok_or("--size must be WxH")?;
                    a.size = Some((w.parse().map_err(|_| "bad width")?, h.parse().map_err(|_| "bad height")?));
                }
                "--seed" => a.seed = Some(val("--seed")?.parse().map_err(|_| "bad seed")?),
                "--radius" => a.radius = Some(val("--radius")?.parse::<i32>().map_err(|_| "bad radius")?.clamp(1, 16)),
                "--user" => a.user = Some(val("--user")?),
                "--vsync" => a.vsync = true,
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn properties_defaults_and_overrides() {
        assert_eq!(Config::from_properties(""), Config::default());
        let c = Config::from_properties("seed=7\nradius=99\nvsync=TRUE\njunk\nradius_x=1");
        assert_eq!((c.seed, c.radius, c.vsync), (7, 16, true));
        assert_eq!(Config::from_properties("seed=abc").seed, 1337);
    }

    #[test]
    fn state_roundtrip_and_rejects_garbage() {
        let st = SaveState { pos: [1.5, 30.0, -4.25], yaw: 0.5, pitch: -0.25 };
        assert_eq!(parse_state(&format_state(&st)), Some(st));
        assert_eq!(parse_state("1 2 3"), None);
        assert_eq!(parse_state("a b c d e"), None);
        assert_eq!(parse_state("1 2 3 4 NaN"), None);
        assert_eq!(parse_state("0 0 0 0 9").unwrap().pitch, 1.55);
    }

    #[test]
    fn args_parse() {
        let a = Args::parse(s(&["--dir", "/x", "--size", "800x600", "--seed", "5", "--user", "Steve", "--vsync"])).unwrap();
        assert_eq!(a.dir, Some(PathBuf::from("/x")));
        assert_eq!((a.size, a.seed, a.vsync, a.user.as_deref()), (Some((800, 600)), Some(5), true, Some("Steve")));
        assert!(Args::parse(s(&["--bogus"])).is_err());
        assert!(Args::parse(s(&["--size", "800"])).is_err());
        assert!(Args::parse(s(&["--seed"])).is_err());
    }
}
