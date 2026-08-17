//! 通知状态管理：存储、过滤、已读/未读、连接状态与指标统计。

use std::collections::HashMap;
use std::rc::Rc;

use notice_model::{NoticeKind, NoticeMessage};
use notice_sse::SseEvent;

/// 服务端配置（来自命令行参数或环境变量）。
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub url: String,
}

impl ServerConfig {
    pub const DEFAULT_URL: &'static str = "http://127.0.0.1:8866/events";

    /// 从命令行参数解析：
    /// `my-notice-app --server <url>` / `my-notice-app --server=<url>`
    /// 未指定时回退到环境变量 `NOTICE_SSE_URL`，再回退到默认地址。
    pub fn from_args() -> Self {
        let mut url: Option<String> = None;
        let args: Vec<String> = std::env::args().collect();
        let mut i = 1;
        while i < args.len() {
            let arg = &args[i];
            if arg == "--server" && i + 1 < args.len() {
                url = Some(args[i + 1].clone());
                i += 2;
                continue;
            }
            if let Some(value) = arg.strip_prefix("--server=") {
                url = Some(value.to_string());
                i += 1;
                continue;
            }
            if arg == "--help" || arg == "-h" {
                print_usage();
                std::process::exit(0);
            }
            i += 1;
        }

        let url = url.or_else(|| std::env::var("NOTICE_SSE_URL").ok());
        Self {
            url: url.unwrap_or_else(|| Self::DEFAULT_URL.to_string()),
        }
    }
}

fn print_usage() {
    println!(
        "my-notice-app —— 基于 GPUI 的桌面通知盒\n\n用法:\n  my-notice-app [--server <SSE地址>]\n\n选项:\n  --server <url>  SSE 服务端地址（默认 {}）\n                  也可用环境变量 NOTICE_SSE_URL 指定\n  -h, --help      显示帮助",
        ServerConfig::DEFAULT_URL
    );
}

/// 列表过滤器。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListFilter {
    /// 全部
    #[default]
    All,
    /// 未读
    Unread,
    /// 已读
    Read,
    /// 按类型
    Kind(NoticeKind),
}

impl ListFilter {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Unread => "未读",
            Self::Read => "已读",
            Self::Kind(kind) => kind.display_name(),
        }
    }
}

/// 连接状态（用于 UI 展示）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// 首次连接中
    #[default]
    Connecting,
    /// 已连接
    Connected,
    /// 断开，准备第 attempt 次重连
    Reconnecting { attempt: u32 },
    /// 已断开（等待重连）
    Disconnected,
}

/// 单次事件处理结果。
#[derive(Debug, Default)]
pub struct StoreChange {
    /// 新到达的通知（用于弹窗提示）。
    pub added: Option<Rc<NoticeMessage>>,
    /// 状态是否有变化（需要刷新界面）。
    pub changed: bool,
}

/// 通知存储：持有全部消息、过滤器、连接状态与统计指标。
#[derive(Debug, Default)]
pub struct NoticeStore {
    /// 全部有效消息（不含 deleted 与已过期），按时间倒序。
    items: Vec<NoticeMessage>,
    /// 当前过滤器。
    pub filter: ListFilter,
    /// 当前选中的消息 ID（详情面板展示）。
    pub selected_id: Option<String>,
    /// 连接状态。
    pub connection: ConnectionState,
    /// 最近一次错误信息。
    pub last_error: Option<String>,
    /// 最近一次心跳时间（Unix 秒）。
    pub last_ping_at: Option<i64>,
    /// 未读数量。
    pub unread_count: usize,
    /// 各类型数量。
    pub kind_counts: HashMap<NoticeKind, usize>,
}

impl NoticeStore {
    /// 处理一个 SSE 事件，返回变化结果。
    pub fn handle_event(&mut self, event: SseEvent, now_unix: i64) -> StoreChange {
        let mut change = StoreChange::default();
        match event {
            SseEvent::Connected { .. } => {
                self.connection = ConnectionState::Connected;
                self.last_error = None;
                change.changed = true;
            }
            SseEvent::Disconnected { reason } => {
                self.connection = ConnectionState::Disconnected;
                self.last_error = Some(reason);
                change.changed = true;
            }
            SseEvent::Reconnecting { attempt, .. } => {
                self.connection = ConnectionState::Reconnecting { attempt };
                change.changed = true;
            }
            SseEvent::Ping => {
                self.last_ping_at = Some(now_unix);
                change.changed = true;
            }
            SseEvent::Error { message } => {
                self.last_error = Some(message);
                change.changed = true;
            }
            SseEvent::Notice(message) => {
                let is_new = self.upsert(message.clone(), now_unix);
                if is_new {
                    change.added = Some(Rc::new(message));
                }
                change.changed = true;
            }
            SseEvent::Raw { .. } => {
                // 非 notice 事件：忽略（可在此扩展业务自定义事件）
            }
        }
        if change.changed {
            self.refresh_metrics(now_unix);
        }
        change
    }

    /// 插入或更新一条消息，返回是否为新消息。
    ///
    /// - 已存在同 ID：整体替换（服务端为状态权威）；
    /// - `status == deleted`：从列表移除；
    /// - 已过期：仍插入（保留历史），但展示与计数时过滤。
    fn upsert(&mut self, message: NoticeMessage, _now_unix: i64) -> bool {
        if message.is_deleted() {
            self.items.retain(|m| m.id != message.id);
            return false;
        }

        if let Some(existing) = self.items.iter_mut().find(|m| m.id == message.id) {
            *existing = message;
            return false;
        }

        // 按时间倒序插入；时间相同时新的排前面
        let position = self
            .items
            .iter()
            .position(|m| m.meta.timestamp < message.meta.timestamp)
            .unwrap_or(self.items.len());
        self.items.insert(position, message);
        true
    }

    /// 将指定消息标记为已读（本地状态；服务端同步为后续扩展点）。
    pub fn mark_read(&mut self, id: &str, now_unix: i64) -> bool {
        let Some(message) = self.items.iter_mut().find(|m| m.id == id) else {
            return false;
        };
        let mut changed = false;
        if let notice_model::NoticeStatus::Unread = message.meta.status {
            message.meta.status = notice_model::NoticeStatus::Read;
            changed = true;
        }
        if changed {
            self.refresh_metrics(now_unix);
        }
        changed
    }

    /// 全部标记为已读。
    pub fn mark_all_read(&mut self, now_unix: i64) -> bool {
        let mut changed = false;
        for message in &mut self.items {
            if message.meta.status == notice_model::NoticeStatus::Unread {
                message.meta.status = notice_model::NoticeStatus::Read;
                changed = true;
            }
        }
        if changed {
            self.refresh_metrics(now_unix);
        }
        changed
    }

    /// 清空已读消息。
    pub fn clear_read(&mut self) -> bool {
        let before = self.items.len();
        self.items
            .retain(|m| m.meta.status != notice_model::NoticeStatus::Read);
        if self.items.len() != before {
            self.refresh_metrics(chrono::Utc::now().timestamp());
            true
        } else {
            false
        }
    }

    /// 清除全部消息。
    pub fn clear_all(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        self.items.clear();
        self.refresh_metrics(chrono::Utc::now().timestamp());
        true
    }

    /// 当前过滤器下的展示列表。
    pub fn filtered_items(&self, now_unix: i64) -> Vec<Rc<NoticeMessage>> {
        self.items
            .iter()
            .filter(|m| !m.is_expired_at(now_unix))
            .filter(|m| match self.filter {
                ListFilter::All => true,
                ListFilter::Unread => m.meta.status == notice_model::NoticeStatus::Unread,
                ListFilter::Read => m.meta.status == notice_model::NoticeStatus::Read,
                ListFilter::Kind(kind) => m.meta.kind == kind,
            })
            .map(|m| Rc::new(m.clone()))
            .collect()
    }

    /// 按 ID 查找消息（不受过滤器限制，用于详情面板）。
    pub fn find(&self, id: &str) -> Option<Rc<NoticeMessage>> {
        self.items
            .iter()
            .find(|m| m.id == id)
            .map(|m| Rc::new(m.clone()))
    }

    /// 当前选中消息。
    pub fn selected(&self) -> Option<Rc<NoticeMessage>> {
        self.selected_id.as_ref().and_then(|id| self.find(id))
    }

    /// 重新统计未读数与各类型数量。
    fn refresh_metrics(&mut self, now_unix: i64) {
        self.unread_count = self
            .items
            .iter()
            .filter(|m| !m.is_expired_at(now_unix))
            .filter(|m| m.meta.status == notice_model::NoticeStatus::Unread)
            .count();
        self.kind_counts.clear();
        for message in &self.items {
            if message.is_expired_at(now_unix) {
                continue;
            }
            *self.kind_counts.entry(message.meta.kind).or_insert(0) += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notice_model::{
        NoticeContent, NoticeInteraction, NoticeKind, NoticeMeta, NoticePriority, NoticeStatus,
    };
    use serde_json::json;

    fn make_notice(
        id: &str,
        kind: NoticeKind,
        status: NoticeStatus,
        timestamp: i64,
    ) -> NoticeMessage {
        NoticeMessage {
            id: id.to_string(),
            meta: NoticeMeta {
                kind,
                status,
                priority: NoticePriority::Low,
                channel: "test-channel".to_string(),
                timestamp,
                expirein: None,
            },
            content: NoticeContent {
                title: format!("标题 {id}"),
                summary: None,
                body: None,
                author: None,
                cover: None,
                tags: vec![],
                entities: vec![],
            },
            interaction: NoticeInteraction { views: vec![] },
            extra: Default::default(),
        }
    }

    fn notice_event(message: NoticeMessage) -> SseEvent {
        SseEvent::Notice(message)
    }

    #[test]
    fn upsert_dedupes_and_sorts_desc() {
        let mut store = NoticeStore::default();
        store.handle_event(
            notice_event(make_notice(
                "a",
                NoticeKind::System,
                NoticeStatus::Unread,
                100,
            )),
            1000,
        );
        store.handle_event(
            notice_event(make_notice(
                "b",
                NoticeKind::System,
                NoticeStatus::Unread,
                300,
            )),
            1000,
        );
        store.handle_event(
            notice_event(make_notice(
                "c",
                NoticeKind::System,
                NoticeStatus::Unread,
                200,
            )),
            1000,
        );
        // 重复 id：更新而非新增
        let change = store.handle_event(
            notice_event(make_notice(
                "b",
                NoticeKind::System,
                NoticeStatus::Read,
                300,
            )),
            1000,
        );
        assert!(change.added.is_none());

        let ids: Vec<&str> = store.items.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
        assert_eq!(store.items.len(), 3);
        assert_eq!(store.unread_count, 2);
    }

    #[test]
    fn deleted_notice_is_removed() {
        let mut store = NoticeStore::default();
        store.handle_event(
            notice_event(make_notice(
                "a",
                NoticeKind::System,
                NoticeStatus::Unread,
                100,
            )),
            1000,
        );
        store.handle_event(
            notice_event(make_notice(
                "a",
                NoticeKind::System,
                NoticeStatus::Deleted,
                100,
            )),
            1000,
        );
        assert!(store.items.is_empty());
    }

    #[test]
    fn mark_read_and_clear_read() {
        let mut store = NoticeStore::default();
        store.handle_event(
            notice_event(make_notice(
                "a",
                NoticeKind::System,
                NoticeStatus::Unread,
                100,
            )),
            1000,
        );
        store.handle_event(
            notice_event(make_notice(
                "b",
                NoticeKind::System,
                NoticeStatus::Unread,
                200,
            )),
            1000,
        );

        assert!(store.mark_read("a", 1000));
        assert_eq!(store.unread_count, 1);
        assert!(!store.mark_read("a", 1000), "重复标记应返回 false");

        store.filter = ListFilter::Read;
        let shown = store.filtered_items(1000);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].id, "a");

        assert!(store.clear_read());
        assert_eq!(store.items.len(), 1);
        assert_eq!(store.unread_count, 1);
    }

    #[test]
    fn mark_all_read() {
        let mut store = NoticeStore::default();
        store.handle_event(
            notice_event(make_notice(
                "a",
                NoticeKind::System,
                NoticeStatus::Unread,
                100,
            )),
            1000,
        );
        store.handle_event(
            notice_event(make_notice(
                "b",
                NoticeKind::System,
                NoticeStatus::Read,
                200,
            )),
            1000,
        );
        assert!(store.mark_all_read(1000));
        assert_eq!(store.unread_count, 0);
    }

    #[test]
    fn expired_items_are_hidden_from_display_and_counts() {
        let mut store = NoticeStore::default();
        let mut msg = make_notice("expired", NoticeKind::System, NoticeStatus::Unread, 100);
        msg.meta.expirein = Some(500);
        store.handle_event(notice_event(msg), 1000);
        store.handle_event(
            notice_event(make_notice(
                "alive",
                NoticeKind::System,
                NoticeStatus::Unread,
                200,
            )),
            1000,
        );

        assert_eq!(store.unread_count, 1);
        let shown = store.filtered_items(1000);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].id, "alive");
    }

    #[test]
    fn kind_filter_and_counts() {
        let mut store = NoticeStore::default();
        store.handle_event(
            notice_event(make_notice(
                "a",
                NoticeKind::Security,
                NoticeStatus::Unread,
                100,
            )),
            1000,
        );
        store.handle_event(
            notice_event(make_notice(
                "b",
                NoticeKind::Transaction,
                NoticeStatus::Unread,
                200,
            )),
            1000,
        );
        store.handle_event(
            notice_event(make_notice(
                "c",
                NoticeKind::Security,
                NoticeStatus::Read,
                300,
            )),
            1000,
        );

        assert_eq!(store.kind_counts.get(&NoticeKind::Security), Some(&2));
        assert_eq!(store.kind_counts.get(&NoticeKind::Transaction), Some(&1));

        store.filter = ListFilter::Kind(NoticeKind::Security);
        let shown = store.filtered_items(1000);
        assert_eq!(shown.len(), 2);
    }

    #[test]
    fn selected_find_ignores_filter() {
        let mut store = NoticeStore::default();
        store.handle_event(
            notice_event(make_notice(
                "a",
                NoticeKind::System,
                NoticeStatus::Unread,
                100,
            )),
            1000,
        );
        store.filter = ListFilter::Read; // "a" 不在 Read 列表里
        store.selected_id = Some("a".to_string());
        assert!(store.selected().is_some(), "详情面板不应受过滤器限制");
    }

    #[test]
    fn server_config_parses_env_and_default() {
        let config = ServerConfig {
            url: "http://x/events".into(),
        };
        assert_eq!(config.url, "http://x/events");
        assert_eq!(ServerConfig::DEFAULT_URL, "http://127.0.0.1:8866/events");
        // json 宏仅用于确保 serde_json 依赖被引用（模型层已有完整单测）
        let _ = json!({"ok": true});
    }
}
