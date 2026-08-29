use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct BackendCfg {
    pub mode: String,   // ws | poll
    pub url: String,
    pub token: String,
    pub poll_secs: u64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PopupCfg {
    pub enabled: bool,
    pub secs: u64,
    pub max_queue: usize,
}

impl Default for PopupCfg {
    fn default() -> Self {
        Self { enabled: true, secs: 6, max_queue: 10 }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FilterCfg {
    pub categories: Vec<String>,  // 空 = 全部
    pub min_priority: String,     // low | normal | high
    pub block: Vec<String>,       // 标题/正文命中即丢弃
    pub only: Vec<String>,        // 仅显示命中关键词的通知（空 = 不过滤）
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct UiCfg {
    pub start_minimized: bool,
}

impl Default for UiCfg {
    fn default() -> Self {
        Self { start_minimized: false }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Config {
    pub backend: BackendCfg,
    pub popup: PopupCfg,
    pub filter: FilterCfg,
    pub ui: UiCfg,
}

impl BackendCfg {
    pub fn defaults() -> Self {
        Self {
            mode: "ws".into(),
            url: "ws://127.0.0.1:9000/ws".into(),
            token: String::new(),
            poll_secs: 5,
        }
    }
}

// 完整默认值（用于生成初始配置文件，避免 BackendCfg 全空）
fn with_defaults(mut c: Config) -> Config {
    if c.backend.url.is_empty() {
        c.backend = BackendCfg::defaults();
    }
    c
}

impl Config {
    fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("config.toml")))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    pub fn load() -> Config {
        let path = Self::path();
        match std::fs::read_to_string(&path) {
            Ok(s) => with_defaults(toml::from_str(&s).unwrap_or_else(|e| {
                eprintln!("config 解析失败({e})，使用默认配置");
                Self::demo()
            })),
            Err(_) => {
                let c = Self::demo();
                let _ = std::fs::write(&path, toml::to_string_pretty(&c).unwrap());
                eprintln!("已生成默认配置: {}", path.display());
                c
            }
        }
    }

    // 出厂演示配置：指向 examples/demo_server.py
    fn demo() -> Config {
        let mut c = Self::default();
        c.backend = BackendCfg::defaults();
        c
    }
}
