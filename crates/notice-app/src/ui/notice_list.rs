//! 通知列表：`ListDelegate` 实现与列表项渲染（含新消息滑入动画）。

use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, Context, ElementId, IntoElement, ParentElement, RenderOnce,
    Styled, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, IndexPath, Selectable, StyledExt as _, h_flex,
    list::{ListDelegate, ListItem, ListState},
    v_flex,
};
use notice_model::{NoticeMessage, NoticePriority};

use crate::ui::{kind_color, kind_icon, priority_color, relative_time};

/// 列表委托：持有当前过滤器下的展示数据，并管理选中行。
#[derive(Default)]
pub struct NoticeListDelegate {
    /// 展示项（已按时间倒序）。
    pub items: Vec<Rc<NoticeMessage>>,
    /// 当前选中行。
    pub selected_index: Option<IndexPath>,
}

impl ListDelegate for NoticeListDelegate {
    type Item = NoticeListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.items.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(&ix) == self.selected_index.as_ref();
        let message = self.items.get(ix.row)?.clone();
        Some(NoticeListItem::new(ix.row, message, selected))
    }

    fn render_empty(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .text_color(cx.theme().muted_foreground.opacity(0.7))
            .child(Icon::new(IconName::Inbox).size_12())
            .child(div().text_sm().child("暂无通知"))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }
}

/// 通知列表项：包裹 `ListItem`，展示类型图标、标题、摘要、时间与优先级。
#[derive(IntoElement)]
pub struct NoticeListItem {
    base: ListItem,
    message: Rc<NoticeMessage>,
    row: usize,
}

impl NoticeListItem {
    pub fn new(row: usize, message: Rc<NoticeMessage>, selected: bool) -> Self {
        Self {
            base: ListItem::new(row).selected(selected),
            message,
            row,
        }
    }

    /// 摘要文本（截断过长的正文）。
    fn summary_text(&self) -> String {
        let text = self
            .message
            .summary_or_body()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("（无摘要）");
        let mut chars = text.chars();
        let mut out: String = chars.by_ref().take(60).collect();
        if chars.next().is_some() {
            out.push('…');
        }
        out
    }
}

impl Selectable for NoticeListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.base = self.base.selected(selected);
        self
    }

    fn is_selected(&self) -> bool {
        self.base.is_selected()
    }
}

impl RenderOnce for NoticeListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let message = &self.message;
        let kind = message.meta.kind;
        let color = kind_color(cx, kind);
        let unread = message.is_unread();
        let priority = message.meta.priority;
        // 预先提取所有展示值，避免 self 被部分移动后无法整体借用
        let title = message.content.title.clone();
        let summary = self.summary_text();
        let time_text = relative_time(message.meta.timestamp);
        let channel = message.meta.channel.clone();
        let row = self.row;

        // 新消息滑入动画：row 号变化时元素重建，动画从头播放
        let animation = Animation::new(std::time::Duration::from_millis(320))
            .with_easing(gpui_component::animation::cubic_bezier(0.25, 0.1, 0.25, 1.));

        self.base
            .px_2()
            .py_2()
            .h(px(76.))
            .child(
                // 唯一子元素：内部横向布局。
                // 注意：ListItem 渲染时会把子元素包裹进一个 w_full 的普通容器，
                // 直接平铺多个子元素会被垂直堆叠（图标跑到标题上方），
                // 因此横向排列必须由这一个 h_flex 自己完成。
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        // 类型图标（带底色圆角块）；div 需显式 flex 才能居中子元素
                        div()
                            .flex()
                            .size(px(34.))
                            .flex_shrink_0()
                            .rounded(cx.theme().radius)
                            .items_center()
                            .justify_center()
                            .bg(color.opacity(0.14))
                            .child(Icon::new(kind_icon(kind)).size(px(20.)).text_color(color)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .overflow_hidden()
                            .gap_1()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .text_sm()
                                            .font_semibold()
                                            .text_color(if unread {
                                                cx.theme().foreground
                                            } else {
                                                cx.theme().foreground.opacity(0.75)
                                            })
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .flex_shrink_0()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(time_text),
                                    ),
                            )
                            .child(
                                div()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(summary),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(color)
                                            .child(kind.display_name()),
                                    )
                                    .child(
                                        div()
                                            .h(px(3.))
                                            .w(px(3.))
                                            .rounded(px(2.))
                                            .bg(cx.theme().muted_foreground.opacity(0.4)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(channel),
                                    )
                                    .when(
                                        !matches!(
                                            priority,
                                            NoticePriority::Low | NoticePriority::Unknown
                                        ),
                                        |this| {
                                            this.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(priority_color(cx, priority))
                                                    .child(priority.display_name()),
                                            )
                                        },
                                    ),
                            ),
                    )
                    .child(
                        // 未读小圆点
                        div()
                            .flex_shrink_0()
                            .size(px(8.))
                            .rounded(px(4.))
                            .bg(if unread {
                                cx.theme().red
                            } else {
                                gpui::transparent_black()
                            }),
                    ),
            )
            .with_animation(
                ElementId::NamedInteger("notice-in".into(), row as u64),
                animation,
                move |this, delta| this.opacity(delta).top(px((1.0 - delta) * 14.0)),
            )
    }
}
