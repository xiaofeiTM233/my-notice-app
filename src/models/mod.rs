//! 数据模型：通知结构体与 SSE 事件信封。

pub mod notification;

pub use notification::{
    Notification, NotificationLevel, RawNotification, SseServerEvent,
    deserialize_flexible_timestamp,
};
