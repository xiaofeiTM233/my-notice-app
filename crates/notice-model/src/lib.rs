//! # notice-model
//!
//! 通知数据模型：对应 `test.json`（Apifox 导出的单条通知数据模型）的 Rust 映射。
//!
//! ```text
//! NotificationPayload (NoticeMessage)
//! ├── id            String           数据 ID
//! ├── meta          NoticeMeta       基本数据
//! │   ├── type        NoticeKind     类型：system/interaction/transaction/security/activity/other
//! │   ├── status      NoticeStatus   状态：unread/read/deleted
//! │   ├── priority    NoticePriority 优先级：low/normal/high/urgent（默认 low）
//! │   ├── channel     String         频道 ID
//! │   ├── timestamp   i64            创建时间（Unix 秒）
//! │   └── expirein    Option<i64>    过期时间（Unix 秒）
//! ├── content       NoticeContent   内容信息
//! │   ├── title       String         标题（必填）
//! │   ├── summary     Option<String> 摘要（列表页展示，缺省取 body）
//! │   ├── body        Option<String> 正文（支持 HTML / Markdown）
//! │   ├── author      Option<NoticeAuthor>
//! │   ├── cover       Option<String> 封面（附件 ID）
//! │   ├── tags        Vec<String>    标签
//! │   └── entities    Vec<String>    附件（附件 ID 列表）
//! ├── interaction   NoticeInteraction
//! │   └── views      Vec<NoticeView> 浏览列表
//! └── extra         HashMap<String, Value> 扩展字段
//! ```
//!
//! 设计约定：
//! - 枚举序列化使用小写（`"system"` / `"unread"` 等），与接口数据一致；
//! - 所有枚举带有 `#[serde(other)]` 的兜底变体，遇到未知取值时解析不失败（向前兼容）；
//! - `priority`、`expirein`、`summary`、`body`、`author`、`tags`、`entities` 等
//!   非必填字段均标记 `#[serde(default)]`。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 单条通知消息（`test.json` 顶层对象）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeMessage {
    /// 数据 ID
    pub id: String,
    /// 基本数据
    pub meta: NoticeMeta,
    /// 内容相关信息
    pub content: NoticeContent,
    /// 互动相关数据
    pub interaction: NoticeInteraction,
    /// 扩展字段，用于存放业务自定义数据
    #[serde(default)]
    pub extra: HashMap<String, Value>,
}

impl NoticeMessage {
    /// 是否未读。
    pub fn is_unread(&self) -> bool {
        self.meta.status == NoticeStatus::Unread
    }

    /// 是否已读。
    pub fn is_read(&self) -> bool {
        self.meta.status == NoticeStatus::Read
    }

    /// 是否已删除。
    pub fn is_deleted(&self) -> bool {
        self.meta.status == NoticeStatus::Deleted
    }

    /// 在给定时刻（Unix 秒）是否已过期。
    ///
    /// `expirein` 语义为“过期时间点”，为空表示永不过期。
    pub fn is_expired_at(&self, now_unix: i64) -> bool {
        self.meta.expirein.is_some_and(|t| t <= now_unix)
    }

    /// 列表展示摘要：优先取 `summary`，缺省回退到 `body`。
    pub fn summary_or_body(&self) -> Option<&str> {
        self.content
            .summary
            .as_deref()
            .or(self.content.body.as_deref())
    }
}

/// 基本数据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeMeta {
    /// 类型（JSON 字段名为 `type`）
    #[serde(rename = "type")]
    pub kind: NoticeKind,
    /// 状态
    pub status: NoticeStatus,
    /// 优先级，默认 low
    #[serde(default)]
    pub priority: NoticePriority,
    /// 频道 ID
    pub channel: String,
    /// 创建时间（Unix 时间戳，秒）
    pub timestamp: i64,
    /// 过期时间（Unix 时间戳，秒），可选
    #[serde(default)]
    pub expirein: Option<i64>,
}

/// 通知类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoticeKind {
    /// 系统
    System,
    /// 互动
    Interaction,
    /// 交易
    Transaction,
    /// 安全
    Security,
    /// 活动
    Activity,
    /// 其他
    Other,
    /// 未知类型（向前兼容兜底）
    #[serde(other)]
    Unknown,
}

impl Default for NoticeKind {
    fn default() -> Self {
        Self::Other
    }
}

impl NoticeKind {
    /// 中文展示名。
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::System => "系统",
            Self::Interaction => "互动",
            Self::Transaction => "交易",
            Self::Security => "安全",
            Self::Activity => "活动",
            Self::Other | Self::Unknown => "其他",
        }
    }
}

/// 通知状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoticeStatus {
    /// 未读
    Unread,
    /// 已读
    Read,
    /// 已删除
    Deleted,
    /// 未知状态（向前兼容兜底，按“已读”处理避免干扰未读计数）
    #[serde(other)]
    Unknown,
}

impl Default for NoticeStatus {
    fn default() -> Self {
        Self::Unread
    }
}

/// 通知优先级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoticePriority {
    /// 低
    Low,
    /// 普通
    Normal,
    /// 高
    High,
    /// 紧急
    Urgent,
    /// 未知优先级（向前兼容兜底，按“低”处理）
    #[serde(other)]
    Unknown,
}

impl Default for NoticePriority {
    fn default() -> Self {
        Self::Low
    }
}

impl NoticePriority {
    /// 中文展示名。
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Low | Self::Unknown => "低",
            Self::Normal => "普通",
            Self::High => "高",
            Self::Urgent => "紧急",
        }
    }

    /// 是否为需要醒目提示的高优先级（高 / 紧急）。
    pub fn is_prominent(&self) -> bool {
        matches!(self, Self::High | Self::Urgent)
    }
}

/// 内容相关信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeContent {
    /// 标题
    pub title: String,
    /// 摘要，用于列表页展示，不填则从 body 中取
    #[serde(default)]
    pub summary: Option<String>,
    /// 正文内容（支持 HTML 或 Markdown）
    #[serde(default)]
    pub body: Option<String>,
    /// 作者信息
    #[serde(default)]
    pub author: Option<NoticeAuthor>,
    /// 封面（附件 ID）
    #[serde(default)]
    pub cover: Option<String>,
    /// 标签列表
    #[serde(default)]
    pub tags: Vec<String>,
    /// 附件列表（附件 ID）
    #[serde(default)]
    pub entities: Vec<String>,
}

/// 作者信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeAuthor {
    /// 作者名称
    #[serde(default)]
    pub name: Option<String>,
    /// 作者头像（附件 ID）
    #[serde(default)]
    pub avatar: Option<String>,
}

/// 互动相关数据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeInteraction {
    /// 浏览列表
    #[serde(default)]
    pub views: Vec<NoticeView>,
}

/// 单条浏览记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeView {
    /// 用户 ID
    #[serde(default)]
    pub id: Option<String>,
    /// 时间（Unix 时间戳，秒）
    pub timestamp: i64,
    /// 日志内容
    #[serde(default)]
    pub body: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 与 test.json 结构完全一致的示例数据（字段命名保持接口原样）。
    const SAMPLE: &str = r#"{
        "id": "n-20260817-0001",
        "meta": {
            "type": "security",
            "status": "unread",
            "priority": "urgent",
            "channel": "ch-account",
            "timestamp": 1755398400,
            "expirein": 1755484800
        },
        "content": {
            "title": "异地登录提醒",
            "summary": "您的账号于 08-17 20:00 在异地设备登录。",
            "body": "您的账号于 08-17 20:00 通过 **Chrome** 在 上海 登录。如非本人操作，请立即修改密码。",
            "author": { "name": "安全中心", "avatar": "att-1001" },
            "tags": ["账号安全", "登录"],
            "cover": null,
            "entities": ["att-1002"]
        },
        "interaction": {
            "views": [
                { "id": "u-001", "timestamp": 1755398500, "body": "安全中心已记录本次事件" }
            ]
        },
        "extra": { "riskLevel": "high" }
    }"#;

    #[test]
    fn parse_sample_matches_test_json_schema() {
        let msg: NoticeMessage = serde_json::from_str(SAMPLE).expect("应当能按 test.json 结构解析");
        assert_eq!(msg.id, "n-20260817-0001");
        assert_eq!(msg.meta.kind, NoticeKind::Security);
        assert_eq!(msg.meta.status, NoticeStatus::Unread);
        assert_eq!(msg.meta.priority, NoticePriority::Urgent);
        assert_eq!(msg.meta.channel, "ch-account");
        assert_eq!(msg.meta.expirein, Some(1755484800));
        assert_eq!(msg.content.title, "异地登录提醒");
        assert_eq!(
            msg.content.author.as_ref().unwrap().name.as_deref(),
            Some("安全中心")
        );
        assert_eq!(msg.content.tags.len(), 2);
        assert_eq!(msg.interaction.views.len(), 1);
        assert_eq!(
            msg.extra.get("riskLevel").unwrap(),
            &Value::String("high".into())
        );
    }

    #[test]
    fn parse_omits_optional_fields() {
        let compact = r#"{
            "id": "n-1",
            "meta": { "type": "system", "status": "read", "channel": "c", "timestamp": 0 },
            "content": { "title": "t" },
            "interaction": {}
        }"#;
        let msg: NoticeMessage = serde_json::from_str(compact).expect("可选字段缺省应能解析");
        assert_eq!(msg.meta.priority, NoticePriority::Low);
        assert!(msg.content.summary.is_none());
        assert!(msg.content.tags.is_empty());
        assert!(msg.interaction.views.is_empty());
        assert!(msg.extra.is_empty());
    }

    #[test]
    fn unknown_enum_values_fall_back() {
        let raw = r#"{
            "id": "n-2",
            "meta": { "type": "future-type", "status": "future-status", "priority": "super", "channel": "c", "timestamp": 0 },
            "content": { "title": "t" },
            "interaction": {}
        }"#;
        let msg: NoticeMessage = serde_json::from_str(raw).expect("未知枚举值应向前兼容");
        assert_eq!(msg.meta.kind, NoticeKind::Unknown);
        assert_eq!(msg.meta.status, NoticeStatus::Unknown);
        assert_eq!(msg.meta.priority, NoticePriority::Unknown); // 未知优先级兜底为 Unknown（展示按低处理）
        assert!(!msg.is_unread());
    }

    #[test]
    fn expire_and_read_helpers() {
        let msg: NoticeMessage = serde_json::from_str(SAMPLE).unwrap();
        assert!(msg.is_unread());
        assert!(!msg.is_read());
        assert!(!msg.is_expired_at(1755399000));
        assert!(msg.is_expired_at(1755484800));
        assert_eq!(
            msg.summary_or_body(),
            Some("您的账号于 08-17 20:00 在异地设备登录。")
        );
    }

    #[test]
    fn serializes_back_to_lowercase_enums() {
        let msg: NoticeMessage = serde_json::from_str(SAMPLE).unwrap();
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["meta"]["type"], "security");
        assert_eq!(json["meta"]["status"], "unread");
        assert_eq!(json["meta"]["priority"], "urgent");
        assert_eq!(json["meta"]["channel"], "ch-account");
    }
}
