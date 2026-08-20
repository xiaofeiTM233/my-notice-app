//! 界面组件：标题条、工具栏、通知卡片列表与空状态。

pub mod header;
pub mod notification_card;
pub mod notification_list;

use chrono::{DateTime, Utc};
pub use header::{render_header, render_toolbar};
pub use notification_card::render_card;
pub use notification_list::render_list;

/// 把创建时间格式化为相对时间（中文）。
///
/// - 1 分钟内 -> 「刚刚」
/// - 1 小时内 -> 「N 分钟前」
/// - 24 小时内 -> 「N 小时前」
/// - 7 天内 -> 「N 天前」
/// - 更早 -> 「MM-DD HH:MM」
pub(crate) fn format_time(dt: DateTime<Utc>) -> String {
    let diff = Utc::now().signed_duration_since(dt);
    if diff.num_seconds() < 60 {
        "刚刚".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{} 分钟前", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{} 小时前", diff.num_hours())
    } else if diff.num_days() < 7 {
        format!("{} 天前", diff.num_days())
    } else {
        dt.format("%m-%d %H:%M").to_string()
    }
}
