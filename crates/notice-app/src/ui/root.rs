//! 主视图：标题栏 + 侧边栏（筛选）+ 通知列表 + 详情面板。
//!
//! 同时负责 SSE 事件轮询（每 120ms 从通道取一次）、通知卡片列表的数据同步、
//! 新消息到达时的右上角弹窗（`WindowExt::push_notification`）。

use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use gpui::{
    App, AppContext, ClickEvent, Context, Entity, Hsla, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, Window,
    div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, IndexPath, Sizable as _, StyledExt as _, TitleBar,
    WindowExt as _,
    badge::Badge,
    button::{Button, ButtonVariants as _},
    h_flex,
    list::{List, ListEvent, ListState},
    notification::Notification,
    v_flex,
};
use notice_model::{NoticeKind, NoticeMessage};
use notice_sse::{SseClient, SseHandle};

use crate::state::{ConnectionState, ListFilter, NoticeStore, ServerConfig};
use crate::ui::detail;
use crate::ui::kind_icon;
use crate::ui::notice_list::NoticeListDelegate;

/// 主视图。
pub struct NoticeRoot {
    store: NoticeStore,
    /// 通知列表（gpui-component List 的状态实体）。
    notice_list: Entity<ListState<NoticeListDelegate>>,
    /// 订阅（保持存活）。
    _subscriptions: Vec<Subscription>,
    /// SSE 事件接收端（GUI 线程轮询）。
    sse_rx: Receiver<notice_sse::SseEvent>,
    /// SSE 连接句柄（保持存活；窗口销毁后线程随通道关闭退出）。
    sse_handle: Option<SseHandle>,
    /// 服务端地址。
    _server_url: String,
}

impl NoticeRoot {
    pub fn new(server: ServerConfig, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (sse_handle, sse_rx) = SseClient::new(server.url.clone())
            .with_backoff(Duration::from_secs(1), Duration::from_secs(30))
            .with_heartbeat_timeout(Duration::from_secs(45))
            .spawn();

        let notice_list = cx.new(|cx| ListState::new(NoticeListDelegate::default(), window, cx));

        let subscriptions = vec![
            cx.subscribe(&notice_list, |this, _, event: &ListEvent, cx| {
                this.on_list_event(event, cx);
            }),
        ];

        let mut root = Self {
            store: NoticeStore::default(),
            notice_list,
            _subscriptions: subscriptions,
            sse_rx,
            sse_handle: Some(sse_handle),
            _server_url: server.url,
        };
        root.spawn_poll_loop(window, cx);
        root
    }

    // ------------------------------------------------------------------
    // SSE 轮询
    // ------------------------------------------------------------------

    /// 后台轮询任务：定时从通道取 SSE 事件并更新界面。
    fn spawn_poll_loop(&self, window: &mut Window, cx: &mut Context<Self>) {
        let weak = cx.entity().downgrade();
        cx.spawn_in(window, async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                if weak
                    .update_in(cx, |this, window, cx| this.poll(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// 单次轮询：消费所有待处理事件。
    fn poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = chrono::Utc::now().timestamp();
        let mut changed = false;
        let mut new_notices: Vec<Rc<NoticeMessage>> = Vec::new();

        while let Ok(event) = self.sse_rx.try_recv() {
            let change = self.store.handle_event(event, now);
            changed |= change.changed;
            if let Some(added) = change.added {
                new_notices.push(added);
            }
        }

        if changed {
            self.sync_list(&mut *cx);
            cx.notify();
            for notice in new_notices {
                self.toast_for(window, &notice, cx);
            }
        }
    }

    /// 新消息到达时在窗口右上角弹出提示。
    fn toast_for(&self, window: &mut Window, message: &NoticeMessage, cx: &mut Context<Self>) {
        let summary = message
            .summary_or_body()
            .map(|s| s.chars().take(80).collect::<String>())
            .unwrap_or_default();
        let note = if message.meta.priority.is_prominent() {
            Notification::warning(summary)
        } else {
            Notification::info(summary)
        }
        .title(message.content.title.clone())
        .autohide(true);
        window.push_notification(note, &mut *cx);
    }

    // ------------------------------------------------------------------
    // 列表同步与事件处理
    // ------------------------------------------------------------------

    /// 将 store 中当前过滤器下的数据同步到 List 委托。
    fn sync_list(&mut self, cx: &mut App) {
        let now = chrono::Utc::now().timestamp();
        let items = self.store.filtered_items(now);
        let selected_index = self.store.selected_id.as_ref().and_then(|id| {
            items
                .iter()
                .position(|m| m.id.as_str() == id.as_str())
                .map(|row| IndexPath::default().row(row))
        });

        self.notice_list.update(cx, |list, cx| {
            let delegate = list.delegate_mut();
            delegate.items = items;
            delegate.selected_index = selected_index;
            cx.notify();
        });
    }

    /// List 组件事件：选中（预览）与确认（打开并标记已读）。
    fn on_list_event(&mut self, event: &ListEvent, cx: &mut App) {
        let now = chrono::Utc::now().timestamp();
        let items = self.store.filtered_items(now);
        match event {
            ListEvent::Confirm(ix) => {
                if let Some(message) = items.get(ix.row) {
                    self.store.selected_id = Some(message.id.clone());
                    if message.is_unread() {
                        self.store.mark_read(&message.id, now);
                    }
                }
                self.sync_list(cx);
                cx.refresh_windows();
            }
            ListEvent::Select(ix) => {
                if let Some(message) = items.get(ix.row) {
                    self.store.selected_id = Some(message.id.clone());
                    self.sync_list(cx);
                    cx.refresh_windows();
                }
            }
            ListEvent::Cancel => {}
        }
    }

    // ------------------------------------------------------------------
    // 渲染
    // ------------------------------------------------------------------

    fn connection_text(&self) -> String {
        match self.store.connection {
            ConnectionState::Connecting => "连接中…".to_string(),
            ConnectionState::Connected => "已连接".to_string(),
            ConnectionState::Reconnecting { attempt } => format!("重连中…(第 {attempt} 次)"),
            ConnectionState::Disconnected => "已断开，等待重连".to_string(),
        }
    }

    fn connection_color(&self, cx: &App) -> Hsla {
        match self.store.connection {
            ConnectionState::Connected => cx.theme().green,
            ConnectionState::Connecting | ConnectionState::Reconnecting { .. } => {
                cx.theme().warning
            }
            ConnectionState::Disconnected => cx.theme().red,
        }
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> TitleBar {
        let unread = self.store.unread_count;
        let status_color = self.connection_color(cx);
        let status_text = self.connection_text();

        TitleBar::new()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .child(
                        Icon::new(IconName::Bell)
                            .small()
                            .text_color(cx.theme().foreground),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("My Notice"),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .px_1()
                            .py_0p5()
                            .rounded_full()
                            .bg(status_color.opacity(0.14))
                            .child(div().size(px(6.)).rounded_full().bg(status_color))
                            .child(div().text_xs().text_color(status_color).child(status_text)),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div().relative().child(
                            Badge::new().count(unread).max(99).child(
                                Button::new("bell-badge")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Bell),
                            ),
                        ),
                    )
                    .child(
                        Button::new("mark-all-read")
                            .small()
                            .ghost()
                            .label("全部已读")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let now = chrono::Utc::now().timestamp();
                                if this.store.mark_all_read(now) {
                                    this.sync_list(cx);
                                    cx.refresh_windows();
                                }
                            })),
                    )
                    .child(
                        Button::new("clear-read")
                            .small()
                            .ghost()
                            .label("清空已读")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.store.clear_read() {
                                    this.store.selected_id = None;
                                    this.sync_list(cx);
                                    cx.refresh_windows();
                                }
                            })),
                    ),
            )
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let filter = self.store.filter;
        let unread = self.store.unread_count;
        let total = self.store.kind_counts.values().sum::<usize>();
        let read = total.saturating_sub(unread);

        let kinds = [
            NoticeKind::System,
            NoticeKind::Interaction,
            NoticeKind::Transaction,
            NoticeKind::Security,
            NoticeKind::Activity,
            NoticeKind::Other,
        ];

        v_flex()
            .w(px(200.))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tokens.popover)
            .p_2()
            .gap_1()
            .child(
                div()
                    .px_2()
                    .pb_1()
                    .pt_2()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child("筛选"),
            )
            .child(self.filter_row(
                cx,
                "filter-all",
                IconName::Inbox,
                "全部",
                total,
                filter == ListFilter::All,
                |this, _, _, cx| {
                    this.store.filter = ListFilter::All;
                    this.sync_list(cx);
                    cx.refresh_windows();
                },
            ))
            .child(self.filter_row(
                cx,
                "filter-unread",
                IconName::Bell,
                "未读",
                unread,
                filter == ListFilter::Unread,
                |this, _, _, cx| {
                    this.store.filter = ListFilter::Unread;
                    this.sync_list(cx);
                    cx.refresh_windows();
                },
            ))
            .child(self.filter_row(
                cx,
                "filter-read",
                IconName::CircleCheck,
                "已读",
                read,
                filter == ListFilter::Read,
                |this, _, _, cx| {
                    this.store.filter = ListFilter::Read;
                    this.sync_list(cx);
                    cx.refresh_windows();
                },
            ))
            .child(
                div()
                    .px_2()
                    .pb_1()
                    .pt_2()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child("类型"),
            )
            .children(kinds.into_iter().map(|kind| {
                let count = self.store.kind_counts.get(&kind).copied().unwrap_or(0);
                let label = kind.display_name();
                let active = filter == ListFilter::Kind(kind);
                let id: &'static str = match kind {
                    NoticeKind::System => "filter-kind-system",
                    NoticeKind::Interaction => "filter-kind-interaction",
                    NoticeKind::Transaction => "filter-kind-transaction",
                    NoticeKind::Security => "filter-kind-security",
                    NoticeKind::Activity => "filter-kind-activity",
                    NoticeKind::Other | NoticeKind::Unknown => "filter-kind-other",
                };
                self.filter_row(
                    cx,
                    id,
                    kind_icon(kind),
                    label,
                    count,
                    active,
                    move |this, _, _, cx| {
                        this.store.filter = ListFilter::Kind(kind);
                        this.sync_list(cx);
                        cx.refresh_windows();
                    },
                )
            }))
            .child(div().flex_1())
            .child(self.render_connection_box(cx))
    }

    /// 单行筛选按钮。
    #[allow(clippy::too_many_arguments)]
    fn filter_row(
        &self,
        cx: &mut Context<Self>,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        count: usize,
        active: bool,
        on_click: impl Fn(&mut NoticeRoot, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let foreground = if active {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };
        let muted = if active {
            cx.theme().accent_foreground
        } else {
            cx.theme().muted_foreground
        };
        let background = if active {
            cx.theme().accent
        } else {
            gpui::transparent_black()
        };
        let hover_background = if active {
            cx.theme().accent
        } else {
            cx.theme().tokens.list_hover
        };

        h_flex()
            .id(id)
            .cursor_pointer()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(cx.theme().radius)
            .items_center()
            .bg(background)
            .hover(move |mut style| {
                style.bg(hover_background);
                style
            })
            .on_click(cx.listener(move |this, ev, window, cx| on_click(this, ev, window, cx)))
            .child(Icon::new(icon).small().text_color(muted))
            .child(div().flex_1().text_sm().text_color(foreground).child(label))
            .child(
                div()
                    .px_1()
                    .rounded_full()
                    .bg(if active {
                        cx.theme().accent_foreground.opacity(0.15)
                    } else {
                        cx.theme().tokens.list_hover
                    })
                    .text_xs()
                    .text_color(muted)
                    .child(count.to_string()),
            )
    }

    fn render_connection_box(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status_color = self.connection_color(cx);
        let status_text = self.connection_text();

        v_flex()
            .gap_1()
            .p_1()
            .rounded(cx.theme().radius)
            .bg(cx.theme().tokens.list_hover)
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .px_1()
                    .child(div().size(px(6.)).rounded_full().bg(status_color))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().foreground)
                            .child(status_text),
                    ),
            )
            .when_some(self.store.last_error.clone(), |this, error| {
                this.child(
                    div()
                        .px_1()
                        .text_xs()
                        .text_color(cx.theme().red)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(error),
                )
            })
    }

    fn render_list_pane(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let now = chrono::Utc::now().timestamp();
        let shown = self.store.filtered_items(now).len();
        let filter_name = self.store.filter.display_name();

        v_flex()
            .w(px(400.))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(format!("通知列表 · {filter_name}")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{shown} 条")),
                    ),
            )
            .child(List::new(&self.notice_list).flex_1().w_full().px_1().py_1())
    }

    fn render_detail_pane(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let message = self.store.selected();

        v_flex()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .p_4()
            .child(if let Some(message) = message {
                v_flex()
                    .size_full()
                    .gap_1()
                    .child(detail::detail_content(&message, cx))
                    .child(
                        h_flex().justify_end().child(
                            Button::new("clear-selection")
                                .xsmall()
                                .ghost()
                                .icon(IconName::Close)
                                .label("关闭")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.selected_id = None;
                                    this.sync_list(cx);
                                    cx.refresh_windows();
                                })),
                        ),
                    )
            } else {
                detail::empty_placeholder(cx)
            })
    }
}

impl Render for NoticeRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_title_bar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(cx))
                    .child(self.render_list_pane(cx))
                    .child(self.render_detail_pane(cx)),
            )
    }
}
