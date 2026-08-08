use crate::model::{Cat, Notif, Prio};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub feed: Feed,
    pub toast: Toast,
    pub filter: Filter,
    pub ui: Ui,
    pub store: Store,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Ws,
    Poll,
    Off,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Feed {
    pub mode: Mode,
    /// mode = ws 时使用，支持 ws:// 与 wss://
    pub url: String,
    /// mode = poll 时使用；服务端可读取 ?since=<unix秒> 做长轮询
    pub poll_url: String,
    pub poll_ms: u64,
    /// 长轮询单次请求的最长等待，应大于服务端 hold 时间
    pub poll_timeout_ms: u64,
    pub retry_ms: u64,
    pub retry_max_ms: u64,
    /// 非空时以 Authorization: Bearer <token> 发送
    pub token: String,
}

impl Default for Feed {
    fn default() -> Self {
        Feed {
            mode: Mode::Ws,
            url: "ws://127.0.0.1:8787/ws".into(),
            poll_url: "http://127.0.0.1:8787/notifications".into(),
            poll_ms: 3000,
            poll_timeout_ms: 35000,
            retry_ms: 2000,
            retry_max_ms: 30000,
            token: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    #[default]
    BottomRight,
    TopRight,
    BottomLeft,
    TopLeft,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Toast {
    pub enabled: bool,
    pub duration_ms: u64,
    pub max_visible: usize,
    pub width: f32,
    pub height: f32,
    pub gap: f32,
    pub margin: f32,
    pub corner: Corner,
    pub anim_ms: i64,
    /// 悬停时取消自动关闭
    pub hold_on_hover: bool,
}

impl Default for Toast {
    fn default() -> Self {
        Toast {
            enabled: true,
            duration_ms: 6000,
            max_visible: 3,
            width: 360.0,
            height: 104.0,
            gap: 10.0,
            margin: 16.0,
            corner: Corner::BottomRight,
            anim_ms: 240,
            hold_on_hover: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    /// 空表示不限制
    pub cats: Vec<Cat>,
    pub min_prio: Prio,
    pub mute_words: Vec<String>,
    pub mute_srcs: Vec<String>,
    pub quiet: Quiet,
}

impl Default for Filter {
    fn default() -> Self {
        Filter {
            cats: vec![],
            min_prio: Prio::Low,
            mute_words: vec![],
            mute_srcs: vec![],
            quiet: Quiet::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Quiet {
    pub enabled: bool,
    pub from: String,
    pub to: String,
    /// 免打扰时段内仍然弹出的最低优先级
    pub pass_prio: Prio,
}

impl Default for Quiet {
    fn default() -> Self {
        Quiet {
            enabled: false,
            from: "22:00".into(),
            to: "08:00".into(),
            pass_prio: Prio::Urgent,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    pub dark: bool,
    pub width: f32,
    pub height: f32,
    pub start_hidden: bool,
}

impl Default for Ui {
    fn default() -> Self {
        Ui {
            dark: false,
            width: 420.0,
            height: 660.0,
            start_hidden: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Store {
    pub max: usize,
    pub persist: bool,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            max: 500,
            persist: true,
        }
    }
}

impl Filter {
    /// 只决定“要不要弹窗”，历史记录一律保留
    pub fn pass(&self, n: &Notif, hour: u32, min: u32) -> bool {
        if !self.cats.is_empty() && !self.cats.contains(&n.cat) {
            return false;
        }
        if n.prio < self.min_prio {
            return false;
        }
        if self.mute_srcs.iter().any(|s| eq_ci(s, &n.src)) {
            return false;
        }
        if !self.mute_words.is_empty() {
            let hay = format!("{} {}", n.title, n.body).to_lowercase();
            if self
                .mute_words
                .iter()
                .any(|w| !w.is_empty() && hay.contains(&w.to_lowercase()))
            {
                return false;
            }
        }
        if self.quiet.enabled
            && in_range(hour * 60 + min, &self.quiet.from, &self.quiet.to)
            && n.prio < self.quiet.pass_prio
        {
            return false;
        }
        true
    }
}

fn eq_ci(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.to_lowercase() == b.to_lowercase()
}

fn hm(s: &str) -> u32 {
    let mut it = s.split(':');
    let h: u32 = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    let m: u32 = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    (h.min(23)) * 60 + m.min(59)
}

/// 支持跨零点，例如 22:00 → 08:00
fn in_range(now: u32, from: &str, to: &str) -> bool {
    let (a, b) = (hm(from), hm(to));
    if a <= b {
        now >= a && now < b
    } else {
        now >= a || now < b
    }
}

pub fn dir() -> PathBuf {
    if let Ok(p) = std::env::var("NOTIFYHUB_DIR") {
        return PathBuf::from(p);
    }
    directories::ProjectDirs::from("", "", "NotifyHub")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn path() -> PathBuf {
    dir().join("config.toml")
}

/// 读取配置；文件不存在时写出一份默认配置，解析失败时回退默认值
pub fn load() -> (Config, Option<String>) {
    let p = path();
    if !p.exists() {
        let c = Config::default();
        let warn = save(&c).err().map(|e| format!("配置写入失败：{e}"));
        return (c, warn);
    }
    match std::fs::read_to_string(&p)
        .map_err(anyhow::Error::from)
        .and_then(|s| toml::from_str::<Config>(&s).map_err(anyhow::Error::from))
    {
        Ok(c) => (c, None),
        Err(e) => (
            Config::default(),
            Some(format!("{} 解析失败，已使用默认配置：{e}", p.display())),
        ),
    }
}

pub fn save(c: &Config) -> Result<()> {
    let p = path();
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(&p, toml::to_string_pretty(c)?)?;
    Ok(())
}
