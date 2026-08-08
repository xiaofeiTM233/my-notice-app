use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cat {
    #[default]
    Message,
    Reminder,
    System,
    Alert,
}

impl Cat {
    pub fn idx(self) -> i32 {
        match self {
            Cat::Message => 0,
            Cat::Reminder => 1,
            Cat::System => 2,
            Cat::Alert => 3,
        }
    }
    pub fn label(self) -> &'static str {
        ["消息", "提醒", "系统", "告警"][self.idx() as usize]
    }
    pub fn mark(self) -> &'static str {
        ["信", "提", "系", "警"][self.idx() as usize]
    }
    /// 通知中心筛选码 2..5 → 分类
    pub fn from_filter(f: i32) -> Option<Cat> {
        match f {
            2 => Some(Cat::Message),
            3 => Some(Cat::Reminder),
            4 => Some(Cat::System),
            5 => Some(Cat::Alert),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Prio {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl Prio {
    pub fn idx(self) -> i32 {
        match self {
            Prio::Low => 0,
            Prio::Normal => 1,
            Prio::High => 2,
            Prio::Urgent => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActKind {
    #[default]
    Url,
    Path,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Act {
    #[serde(rename = "type", default)]
    pub kind: ActKind,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notif {
    #[serde(default = "gen_id")]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default = "def_src")]
    pub src: String,
    #[serde(default, alias = "category")]
    pub cat: Cat,
    #[serde(default, alias = "priority")]
    pub prio: Prio,
    #[serde(default = "now", alias = "timestamp")]
    pub ts: i64,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub act: Option<Act>,
}

impl Notif {
    /// 供本地生成（欢迎消息、错误提示等）
    pub fn local(title: &str, body: &str, cat: Cat, prio: Prio) -> Self {
        Notif {
            id: gen_id(),
            title: title.into(),
            body: body.into(),
            src: "NotifyHub".into(),
            cat,
            prio,
            ts: now(),
            read: false,
            act: None,
        }
    }

    pub fn hit(&self, q: &str) -> bool {
        if q.is_empty() {
            return true;
        }
        let q = q.to_lowercase();
        self.title.to_lowercase().contains(&q)
            || self.body.to_lowercase().contains(&q)
            || self.src.to_lowercase().contains(&q)
    }
}

/// 后端可以推单条、数组，或 {"notifications": [...]}
/// 变体顺序不能调整：Notif 全字段带默认值，放最后才不会吞掉前两种。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Payload {
    Wrapped { notifications: Vec<Notif> },
    Many(Vec<Notif>),
    One(Box<Notif>),
}

impl Payload {
    pub fn into_vec(self) -> Vec<Notif> {
        match self {
            Payload::Wrapped { notifications } => notifications,
            Payload::Many(v) => v,
            Payload::One(n) => vec![*n],
        }
    }
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn def_src() -> String {
    "未知来源".into()
}

pub fn gen_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", now(), n)
}
