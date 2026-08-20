//! 应用根视图：持有通知列表与 SSE 客户端，负责事件分发与整体渲染。

use gpui::{AnyWindowHandle, Context, Render, SharedString, Window, div, prelude::*};
use gpui_component::{
    ActiveTheme as _, Theme, ThemeMode, WindowExt as _, notification::Notification as Toast,
};

use crate::config::Config;
use crate::models::notification::{Notification, SseServerEvent};
use crate::sse::{ConnectionState, SseClient, SseClientEvent, SseMessage};
use crate::ui::{render_header, render_list, render_toolbar};

/// 通知筛选条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    /// 全部通知。
    #[default]
    All,
    /// 仅未读。
    Unread,
    /// 仅已读。
    Read,
}

/// 应用根视图状态。
pub struct NotificationApp {
    /// 通知列表（新通知在前）。
    pub(crate) notifications: Vec<Notification>,
    /// SSE 连接状态。
    pub(crate) connection_state: ConnectionState,
    /// 当前主题模式。
    pub(crate) theme_mode: ThemeMode,
    /// 列表筛选条件。
    pub(crate) filter: Filter,
    /// SSE 客户端（持有后台工作线程）。
    pub(crate) sse: SseClient,
    /// 主窗口句柄（用于弹窗通知 / 主题切换）。
    pub(crate) window: Option<AnyWindowHandle>,
}

/// 弹窗通知的类型标记（保证按通知 id 去重替换）。
struct NoticeToast;

impl NotificationApp {
    /// 构建应用视图：创建 SSE 客户端并启动事件循环。
    pub fn new(config: Config, window: AnyWindowHandle, cx: &mut Context<Self>) -> Self {
        let (sse, rx) = SseClient::new(config.server_url.clone());

        let mut app = Self {
            notifications: Vec::new(),
            connection_state: ConnectionState::Disconnected,
            theme_mode: cx.theme().mode,
            filter: Filter::All,
            sse,
            window: Some(window),
        };

        // SSE 事件循环：后台线程推送事件，这里把它们带回到视图上下文。
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                let keep_going = cx
                    .update(|cx| match this.upgrade() {
                        Some(entity) => {
                            entity.update(cx, |app, cx| app.on_sse_event(event, cx));
                            true
                        }
                        None => false,
                    })
                    .unwrap_or_default();
                if !keep_going {
                    break;
                }
            }
        })
        .detach();

        app.sse.connect();
        app
    }

    /// 未读通知数量。
    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|note| !note.read).count()
    }

    /// 按当前筛选条件返回可见通知。
    pub fn visible_notifications(&self) -> impl Iterator<Item = &Notification> {
        let filter = self.filter;
        self.notifications.iter().filter(move |note| match filter {
            Filter::All => true,
            Filter::Unread => !note.read,
            Filter::Read => note.read,
        })
    }

    /// 处理来自 SSE 客户端的事件。
    fn on_sse_event(&mut self, event: SseClientEvent, cx: &mut Context<Self>) {
        match event {
            SseClientEvent::State(state) => self.connection_state = state,
            SseClientEvent::Message(message) => self.on_sse_message(message, cx),
        }
        cx.notify();
    }

    /// 处理一条完整的 SSE 报文。
    fn on_sse_message(&mut self, message: SseMessage, cx: &mut Context<Self>) {
        match SseServerEvent::parse(message.event.as_deref(), &message.data) {
            Ok(Some(SseServerEvent::Notification(note))) => self.add_notification(note, cx),
            Ok(Some(SseServerEvent::Read { id })) => self.mark_read(&id),
            Ok(Some(SseServerEvent::ReadAll)) => self.mark_all_read(),
            Ok(Some(SseServerEvent::Clear)) => self.clear_all(),
            Ok(Some(SseServerEvent::Connected { .. })) | Ok(Some(SseServerEvent::Ping)) => {}
            Ok(None) => {}
            Err(err) => log::warn!("SSE 报文解析失败: {err}"),
        }
    }

    /// 新增一条通知；若 id 已存在则原地更新。同时触发弹窗动画。
    fn add_notification(&mut self, note: Notification, cx: &mut Context<Self>) {
        if let Some(existing) = self.notifications.iter_mut().find(|n| n.id == note.id) {
            *existing = note.clone();
        } else {
            self.notifications.insert(0, note.clone());
        }
        self.show_toast(&note, cx);
    }

    /// 在窗口右上角弹出轻提示（带进入/退出动画）。
    fn show_toast(&mut self, note: &Notification, cx: &mut Context<Self>) {
        if let Some(window) = self.window {
            let toast = Toast::new()
                .id1::<NoticeToast>(SharedString::from(note.id.clone()))
                .title(note.title.clone())
                .message(note.message.clone())
                .with_type(note.level.to_component());
            let _ = window.update(cx, |_view, window, cx| {
                window.push_notification(toast, cx);
            });
        }
    }

    /// 将指定通知标记为已读。
    pub fn mark_read(&mut self, id: &str) {
        if let Some(note) = self.notifications.iter_mut().find(|note| note.id == id) {
            note.read = true;
        }
    }

    /// 将所有通知标记为已读。
    pub fn mark_all_read(&mut self) {
        for note in &mut self.notifications {
            note.read = true;
        }
    }

    /// 删除指定通知。
    pub fn remove(&mut self, id: &str) {
        self.notifications.retain(|note| note.id != id);
    }

    /// 清空全部通知。
    pub fn clear_all(&mut self) {
        self.notifications.clear();
    }

    /// 切换浅色/深色主题。
    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        let new_mode = match self.theme_mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        self.theme_mode = new_mode;
        if let Some(window) = self.window {
            let _ = window.update(cx, |_view, window, cx| {
                Theme::change(new_mode, Some(window), cx);
            });
        }
    }

    /// 请求服务端推送一条测试通知（POST `{server_url}/send`）。
    pub fn send_test(&mut self) {
        let payload = serde_json::json!({
            "title": "测试通知",
            "message": "这是一条由「发送测试」按钮生成的本地通知。",
            "type": "success",
            "source": "local",
        });
        if let Err(err) = self.sse.send_test_notification(payload) {
            log::warn!("发送测试通知失败: {err}");
        }
    }
}

impl Render for NotificationApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .child(render_header(self, cx))
            .child(render_toolbar(self, cx))
            .child(render_list(self, cx))
    }
}
