//! 通知盒 UI：布局、通知列表、详情面板与公共渲染辅助。

pub mod detail;
pub mod notice_list;
pub mod root;

use chrono::{DateTime, Local};
use gpui::{App, Hsla};
use gpui_component::{ActiveTheme as _, IconName};
use notice_model::{NoticeKind, NoticePriority};

/// 类型 → 图标。
pub fn kind_icon(kind: NoticeKind) -> IconName {
    match kind {
        NoticeKind::System => IconName::Settings2,
        NoticeKind::Interaction => IconName::ThumbsUp,
        NoticeKind::Transaction => IconName::CircleCheck,
        NoticeKind::Security => IconName::TriangleAlert,
        NoticeKind::Activity => IconName::Star,
        NoticeKind::Other | NoticeKind::Unknown => IconName::File,
    }
}

/// 类型 → 强调色。
pub fn kind_color(cx: &App, kind: NoticeKind) -> Hsla {
    match kind {
        NoticeKind::System => cx.theme().foreground,
        NoticeKind::Interaction => cx.theme().info,
        NoticeKind::Transaction => cx.theme().green,
        NoticeKind::Security => cx.theme().red,
        NoticeKind::Activity => cx.theme().warning,
        NoticeKind::Other | NoticeKind::Unknown => cx.theme().muted_foreground,
    }
}

/// 优先级 → 强调色。
pub fn priority_color(cx: &App, priority: NoticePriority) -> Hsla {
    match priority {
        NoticePriority::Urgent => cx.theme().red,
        NoticePriority::High => cx.theme().warning,
        NoticePriority::Normal => cx.theme().blue,
        NoticePriority::Low | NoticePriority::Unknown => cx.theme().muted_foreground,
    }
}

/// Unix 秒 → 本地时间格式串 `MM-dd HH:mm`。
pub fn format_time(unix_seconds: i64) -> String {
    let Some(dt) = DateTime::<Local>::from_timestamp(unix_seconds, 0) else {
        return String::new();
    };
    dt.format("%m-%d %H:%M").to_string()
}

/// Unix 秒 → 完整时间格式串 `yyyy-MM-dd HH:mm:ss`。
pub fn format_full_time(unix_seconds: i64) -> String {
    let Some(dt) = DateTime::<Local>::from_timestamp(unix_seconds, 0) else {
        return String::new();
    };
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 相对时间（用于列表项）。
pub fn relative_time(unix_seconds: i64) -> String {
    let Some(dt) = DateTime::<Local>::from_timestamp(unix_seconds, 0) else {
        return String::new();
    };
    let now = Local::now();
    let diff = now.signed_duration_since(dt);
    let secs = diff.num_seconds();
    if secs < 0 {
        return "刚刚".to_string();
    }
    if secs < 60 {
        return "刚刚".to_string();
    }
    if secs < 3600 {
        return format!("{} 分钟前", secs / 60);
    }
    if secs < 86400 {
        return format!("{} 小时前", secs / 3600);
    }
    if secs < 86400 * 7 {
        return format!("{} 天前", secs / 86400);
    }
    dt.format("%Y-%m-%d").to_string()
}
