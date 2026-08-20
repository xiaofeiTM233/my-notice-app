//! 通知数据模型：SSE 服务端推送的消息体解析与映射。
//!
//! 服务端每条 SSE 消息由 `event:`（事件名）与 `data:`（JSON 报文）组成，
//! 本模块负责把 JSON 报文解析为强类型的 [`Notification`]，并把
//! [`SseServerEvent`] 事件信封映射为应用内部可消费的事件。

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// 通知级别（决定卡片图标与配色）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationLevel {
    /// 容错解析字符串级别，未知值回退为 `Info`。
    pub fn parse<S: AsRef<str>>(value: S) -> Self {
        match value.as_ref().trim().to_ascii_lowercase().as_str() {
            "success" | "ok" | "done" => Self::Success,
            "warning" | "warn" => Self::Warning,
            "error" | "fail" | "failed" => Self::Error,
            _ => Self::Info,
        }
    }

    /// 映射为 gpui-component 的弹窗通知类型。
    pub fn to_component(self) -> gpui_component::notification::NotificationType {
        match self {
            Self::Info => gpui_component::notification::NotificationType::Info,
            Self::Success => gpui_component::notification::NotificationType::Success,
            Self::Warning => gpui_component::notification::NotificationType::Warning,
            Self::Error => gpui_component::notification::NotificationType::Error,
        }
    }
}

/// 通知数据模型。
///
/// 字段均做了容错处理：
/// - `created_at` 兼容「毫秒/秒级时间戳」与「RFC3339/ISO8601 字符串」；
/// - `type` 是 Rust 关键字，序列化时以 `type` 为键（服务端友好），
///   解析时额外兼容 `level`、`severity` 等别名；
/// - 可选字段（`source`/`link`/`metadata`/`read`）缺失时使用默认值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Notification {
    /// 通知唯一 ID（服务端生成）。
    pub id: String,
    /// 通知级别。
    #[serde(rename = "type", alias = "level", alias = "severity", default)]
    pub level: NotificationLevel,
    /// 标题。
    pub title: String,
    /// 正文内容。
    pub message: String,
    /// 来源（应用/服务名），默认空。
    #[serde(default)]
    pub source: String,
    /// 创建时间，默认当前时间。
    #[serde(
        default = "default_timestamp",
        deserialize_with = "deserialize_flexible_timestamp"
    )]
    pub created_at: DateTime<Utc>,
    /// 可选的跳转链接。
    #[serde(default)]
    pub link: Option<String>,
    /// 已读标记（应用侧本地状态，默认未读）。
    #[serde(default)]
    pub read: bool,
    /// 服务端附加的扩展字段，透传保留。
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl Notification {
    /// 构建一条新通知（用于本地生成或测试）。
    pub fn new(
        id: impl Into<String>,
        level: NotificationLevel,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            level,
            title: title.into(),
            message: message.into(),
            source: String::new(),
            created_at: Utc::now(),
            link: None,
            read: false,
            metadata: None,
        }
    }
}

fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// 兼容「数字时间戳（秒/毫秒）」与「RFC3339 / ISO8601 字符串」的反序列化器。
pub fn deserialize_flexible_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(num) => {
            if let Some(secs) = num.as_i64() {
                if secs > 10_000_000_000 {
                    // 毫秒时间戳
                    Ok(DateTime::<Utc>::from_timestamp_millis(secs).unwrap_or_else(Utc::now))
                } else {
                    Ok(DateTime::<Utc>::from_timestamp(secs, 0).unwrap_or_else(Utc::now))
                }
            } else if let Some(f) = num.as_f64() {
                Ok(DateTime::<Utc>::from_timestamp(f as i64, 0).unwrap_or_else(Utc::now))
            } else {
                Err(serde::de::Error::custom("invalid numeric timestamp"))
            }
        }
        Value::String(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                Ok(dt.with_timezone(&Utc))
            } else if let Ok(dt) = chrono::DateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f %z") {
                Ok(dt.with_timezone(&Utc))
            } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f")
            {
                Ok(dt.and_utc())
            } else {
                Err(serde::de::Error::custom(format!(
                    "unrecognized timestamp format: {s}"
                )))
            }
        }
        Value::Null => Ok(Utc::now()),
        other => Err(serde::de::Error::custom(format!(
            "unsupported timestamp value: {other}"
        ))),
    }
}

/// SSE 服务端事件信封（`event:` 字段与 `data:` JSON 的组合）。
#[derive(Debug, Clone, PartialEq)]
pub enum SseServerEvent {
    /// 收到一条新通知（对应事件名 `notification`，或未声明事件名的裸通知报文）。
    Notification(Notification),
    /// 服务端要求将指定通知标记为已读（事件名 `read`）。
    Read { id: String },
    /// 服务端要求将所有通知标记为已读（事件名 `read_all`）。
    ReadAll,
    /// 服务端要求清空通知列表（事件名 `clear`）。
    Clear,
    /// 连接建立成功的握手事件（事件名 `connected`）。
    Connected { server_time: Option<String> },
    /// 心跳事件（事件名 `ping` / 注释帧）。
    Ping,
}

impl SseServerEvent {
    /// 根据 SSE 事件名与 data 报文解析为应用事件。
    ///
    /// - 返回 `Ok(None)` 表示该帧无需处理（如注释、空 data）。
    /// - 返回 `Err` 表示解析失败，调用方应记日志并继续。
    pub fn parse(event: Option<&str>, data: &str) -> anyhow::Result<Option<Self>> {
        let event = event.unwrap_or("").trim();
        let data = data.trim();

        if data.is_empty() {
            return Ok(None);
        }

        let json: Value = serde_json::from_str(data)?;

        let evt = match event {
            "" => Self::parse_default(&json)?,
            "notification" | "notice" | "message" => {
                Self::Notification(parse_notification_value(&json)?)
            }
            "read" => Self::Read {
                id: parse_required_string(&json, &["id", "notification_id"])?,
            },
            "read_all" | "readAll" => Self::ReadAll,
            "clear" | "clear_all" | "clearAll" => Self::Clear,
            "connected" | "ready" => Self::Connected {
                server_time: json
                    .get("server_time")
                    .or_else(|| json.get("time"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
            },
            "ping" | "heartbeat" | "keepalive" => Self::Ping,
            _ => Self::parse_default(&json)?,
        };

        Ok(Some(evt))
    }

    /// 事件名未知时的默认策略：优先尝试解析为通知报文；
    /// 否则解析为 `{ event: "..." }` 的结构化信封。
    fn parse_default(json: &Value) -> anyhow::Result<Self> {
        if is_notification_payload(json) {
            Ok(Self::Notification(parse_notification_value(json)?))
        } else if let Some(name) = json.get("event").and_then(Value::as_str) {
            Self::parse(
                Some(name),
                &json.get("data").map(|v| v.to_string()).unwrap_or_default(),
            )
            .map(|evt| evt.unwrap_or(Self::Ping))
        } else {
            anyhow::bail!("unsupported SSE payload: {json}")
        }
    }
}

/// 判断 JSON 是否具备通知报文的最小特征（存在 id 与 message/title）。
fn is_notification_payload(json: &Value) -> bool {
    json.get("id").is_some() && (json.get("message").is_some() || json.get("title").is_some())
}

fn parse_notification_value(json: &Value) -> anyhow::Result<Notification> {
    serde_json::from_value(json.clone()).map_err(|e| anyhow::anyhow!("invalid notification: {e}"))
}

fn parse_required_string(json: &Value, keys: &[&str]) -> anyhow::Result<String> {
    for key in keys {
        if let Some(v) = json.get(*key) {
            if let Some(s) = v.as_str() {
                return Ok(s.to_owned());
            }
            if let Some(s) = v.as_i64() {
                return Ok(s.to_string());
            }
            return Ok(v.to_string());
        }
    }
    anyhow::bail!("missing required field: {:?}", keys)
}

/// 原始通知在服务端可能附带额外字段，通过 serde 的未知字段策略默认忽略，
/// 这里显式保留 `raw` 原始报文，便于上层调试。
#[derive(Debug, Clone)]
pub struct RawNotification {
    pub notification: Notification,
    pub raw: HashMap<String, Value>,
}

impl RawNotification {
    pub fn parse(data: &str) -> anyhow::Result<Self> {
        let raw: HashMap<String, Value> = serde_json::from_str(data)?;
        let notification: Notification = serde_json::from_value(
            raw.iter()
                .fold(serde_json::Map::new(), |mut map, (k, v)| {
                    map.insert(k.clone(), v.clone());
                    map
                })
                .into(),
        )?;
        Ok(Self { notification, raw })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_notification_with_iso_timestamp() {
        let raw = json!({
            "id": "n-1",
            "type": "success",
            "title": "构建完成",
            "message": "release build 已通过",
            "source": "ci",
            "created_at": "2026-08-17T08:00:00Z",
            "link": "https://example.com/build/1",
            "read": false
        });

        let note: Notification = serde_json::from_value(raw).unwrap();
        assert_eq!(note.level, NotificationLevel::Success);
        assert_eq!(note.title, "构建完成");
        assert_eq!(note.source, "ci");
        assert_eq!(
            note.created_at,
            DateTime::parse_from_rfc3339("2026-08-17T08:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
        );
        assert!(!note.read);
    }

    #[test]
    fn parse_notification_with_epoch_timestamp() {
        let raw = json!({
            "id": "n-2",
            "type": "warning",
            "title": "磁盘告警",
            "message": "磁盘使用率超过 90%",
            "created_at": 1_720_000_000_000i64
        });
        let note: Notification = serde_json::from_value(raw).unwrap();
        assert_eq!(note.level, NotificationLevel::Warning);
        assert_eq!(
            note.created_at,
            DateTime::from_timestamp_millis(1_720_000_000_000i64).unwrap()
        );
    }

    #[test]
    fn parse_notification_with_level_alias() {
        let raw = json!({
            "id": "n-3",
            "level": "error",
            "title": "服务异常",
            "message": "api 服务不可达",
            "created_at": null
        });
        let note: Notification = serde_json::from_value(raw).unwrap();
        assert_eq!(note.level, NotificationLevel::Error);
        assert!(note.created_at <= Utc::now());
    }

    #[test]
    fn missing_optional_fields_use_defaults() {
        let raw = json!({
            "id": "n-4",
            "title": "无正文标题",
            "message": "hello"
        });
        let note: Notification = serde_json::from_value(raw).unwrap();
        assert_eq!(note.source, "");
        assert_eq!(note.link, None);
        assert!(!note.read);
        assert_eq!(note.level, NotificationLevel::Info);
    }

    #[test]
    fn parse_sse_notification_event() {
        let data = r#"{"id":"n-5","type":"info","title":"t","message":"m"}"#;
        let event = SseServerEvent::parse(Some("notification"), data)
            .unwrap()
            .unwrap();
        match event {
            SseServerEvent::Notification(n) => assert_eq!(n.id, "n-5"),
            other => panic!("expected notification, got {other:?}"),
        }
    }

    #[test]
    fn parse_sse_read_event() {
        let event = SseServerEvent::parse(Some("read"), r#"{"id":"n-9"}"#)
            .unwrap()
            .unwrap();
        assert_eq!(event, SseServerEvent::Read { id: "n-9".into() });
    }

    #[test]
    fn parse_sse_connected_event() {
        let event = SseServerEvent::parse(
            Some("connected"),
            r#"{"server_time":"2026-08-17T00:00:00Z"}"#,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(event, SseServerEvent::Connected { .. }));
    }

    #[test]
    fn parse_sse_ping_event() {
        let event = SseServerEvent::parse(Some("ping"), r#"{}"#)
            .unwrap()
            .unwrap();
        assert_eq!(event, SseServerEvent::Ping);
    }

    #[test]
    fn parse_raw_notification_without_event_name() {
        let data = r#"{"id":"n-6","title":"直接推送","message":"无事件名的裸报文","type":"error"}"#;
        let event = SseServerEvent::parse(None, data).unwrap().unwrap();
        assert!(matches!(event, SseServerEvent::Notification(_)));
    }

    #[test]
    fn empty_data_is_ignored() {
        assert!(
            SseServerEvent::parse(Some("notification"), " ")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn malformed_data_returns_error() {
        assert!(SseServerEvent::parse(Some("notification"), "{ not json").is_err());
    }

    #[test]
    fn level_parse_fallback() {
        assert_eq!(
            NotificationLevel::parse("SUCCESS"),
            NotificationLevel::Success
        );
        assert_eq!(
            NotificationLevel::parse("  warn "),
            NotificationLevel::Warning
        );
        assert_eq!(NotificationLevel::parse("unknown"), NotificationLevel::Info);
    }

    #[test]
    fn raw_notification_keeps_extra_fields() {
        let data = r#"{"id":"n-7","title":"t","message":"m","extra":{"k":1}}"#;
        let raw = RawNotification::parse(data).unwrap();
        assert_eq!(raw.notification.id, "n-7");
        assert!(raw.raw.contains_key("extra"));
    }
}
