//! 通知列表区：按筛选条件渲染卡片，空列表时显示空状态。

use gpui::{AnyElement, Context, IntoElement, ParentElement as _, Styled, div};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _, h_flex, scroll::ScrollableElement, v_flex,
};

use crate::app::{Filter, NotificationApp};
use crate::ui::render_card;

/// 渲染通知列表（含滚动条）。
pub fn render_list(app: &NotificationApp, cx: &mut Context<NotificationApp>) -> AnyElement {
    let theme = cx.theme().clone();
    let visible: Vec<_> = app.visible_notifications().collect();

    if visible.is_empty() {
        return empty_state(app.filter, theme).into_any_element();
    }

    let cards: Vec<AnyElement> = visible
        .into_iter()
        .map(|note| render_card(note, cx).into_any_element())
        .collect();

    h_flex()
        .flex_1()
        .flex_col()
        .gap_2()
        .px_2()
        .py_2()
        .overflow_y_scrollbar()
        .children(cards)
        .into_any_element()
}

/// 空状态：大图标 + 文案。
fn empty_state(filter: Filter, theme: gpui_component::Theme) -> impl IntoElement {
    let text = match filter {
        Filter::All => "暂无通知",
        Filter::Unread => "没有未读通知",
        Filter::Read => "没有已读通知",
    };

    v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .child(Icon::new(IconName::Inbox).large().text_color(theme.muted))
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text),
        )
}
