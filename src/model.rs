use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};

pub static CNT: AtomicU64 = AtomicU64::new(1);

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Msg {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub ts: i64,
}

pub fn cat_code(s: &str) -> i32 {
    match s {
        "reminder" => 1,
        "system" => 2,
        _ => 0,
    }
}

pub fn prio_code(s: &str) -> i32 {
    match s {
        "normal" => 1,
        "high" | "urgent" => 2,
        _ => 0,
    }
}

pub fn rel_time(ts: i64) -> String {
    let d = (now() - ts).max(0);
    if d < 60 {
        "刚刚".into()
    } else if d < 3600 {
        format!("{} 分钟前", d / 60)
    } else if d < 86400 {
        format!("{} 小时前", d / 3600)
    } else if d < 172800 {
        "昨天".into()
    } else {
        format!("{} 天前", d / 86400)
    }
}

pub fn to_item(m: Msg) -> crate::NItem {
    let ts = if m.ts > 0 { m.ts } else { now() };
    let id = if m.id.is_empty() {
        format!("{:x}-{:x}", ts, CNT.fetch_add(1, Ordering::Relaxed))
    } else {
        m.id
    };
    crate::NItem {
        id: id.into(),
        title: m.title.into(),
        body: m.body.into(),
        cat: cat_code(&m.category),
        prio: prio_code(&m.priority),
        ts: ts as i32,
        read: false,
        rel: rel_time(ts).into(),
        url: m.url.into(),
    }
}

// 弹窗资格过滤（通知中心始终保留全部）
pub fn passes(f: &crate::config::FilterCfg, it: &crate::NItem) -> bool {
    if !f.categories.is_empty() && !f.categories.iter().any(|c| cat_code(c) == it.cat) {
        return false;
    }
    if it.prio < prio_code(&f.min_priority) {
        return false;
    }
    let text = format!("{} {}", it.title, it.body).to_lowercase();
    if f.block.iter().any(|k| text.contains(&k.to_lowercase())) {
        return false;
    }
    if !f.only.is_empty() && !f.only.iter().any(|k| text.contains(&k.to_lowercase())) {
        return false;
    }
    true
}
