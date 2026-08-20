//! 单条通知卡片：级别图标、标题、正文、来源与删除操作。

use gpui::{
    Context, FontWeight, InteractiveElement, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _, button::Button, button::ButtonVariants as _, h_flex,
};

use crate::app::NotificationApp;
use crate::models::notification::{Notification, NotificationLevel};
use crate::ui::format_time;

/// 渲染一条通知卡片。
///
/// - 未读：卡片底色更醒目、标题加粗、左侧级别图标着色；
/// - 点击卡片标记为已读；右上角可删除（阻止冒泡）。
pub fn render_card(note: &Notification, cx: &mut Context<NotificationApp>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let (icon_name, level_color) = level_icon(note.level, &theme);
    let id = SharedString::from(note.id.clone());
    let read = note.read;
    let time = format_time(note.created_at);
    let title = note.title.clone();
    let message = note.message.clone();
    let source = note.source.clone();
    let link = note.link.clone();
    let card_bg = if read { theme.list } else { theme.group_box };
    let card_border = if read {
        theme.border
    } else {
        level_color.alpha(0.35)
    };
    let hover_bg = theme.list_hover;
    let text_color = theme.muted_foreground;
    let link_color = theme.link;
    let icon_color = level_color;

    let click_id = id.clone();
    div()
        .id(id.clone())
        .cursor_pointer()
        .flex()
        .flex_col()
        .gap_1p5()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(card_bg)
        .border_1()
        .border_color(card_border)
        .hover(move |style| style.bg(hover_bg))
        .on_click(cx.listener(move |app, _ev, _window, cx| {
            app.mark_read(&click_id);
            cx.notify();
        }))
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(Icon::new(icon_name).xsmall().text_color(icon_color))
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .font_weight(if read {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::SEMIBOLD
                        })
                        .line_clamp(1)
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(text_color)
                        .flex_shrink_0()
                        .child(time),
                ),
        )
        .child(
            div()
                .text_sm()
                .text_color(text_color)
                .line_clamp(3)
                .child(message),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .justify_between()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .when_some(Some(source), |this, source| {
                            this.child(div().text_xs().text_color(text_color).child(source))
                        })
                        .when_some(link, |this, _link| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(link_color)
                                    .cursor_pointer()
                                    .child("查看"),
                            )
                        }),
                )
                .child(delete_button(&id, cx)),
        )
}

/// 删除按钮：阻止点击冒泡，避免同时触发卡片「标记已读」。
fn delete_button(id: &SharedString, cx: &mut Context<NotificationApp>) -> impl IntoElement {
    let btn_id = SharedString::from(format!("delete-{id}"));
    let id = id.clone();
    Button::new(btn_id)
        .icon(IconName::Delete)
        .ghost()
        .xsmall()
        .tooltip("删除这条通知")
        .on_click(cx.listener(move |app, _ev, _window, cx| {
            cx.stop_propagation();
            app.remove(&id);
            cx.notify();
        }))
}

/// 根据级别返回（图标名，主题色）。
fn level_icon(level: NotificationLevel, theme: &gpui_component::Theme) -> (IconName, gpui::Hsla) {
    match level {
        NotificationLevel::Info => (IconName::Info, theme.info),
        NotificationLevel::Success => (IconName::CircleCheck, theme.success),
        NotificationLevel::Warning => (IconName::TriangleAlert, theme.warning),
        NotificationLevel::Error => (IconName::CircleX, theme.danger),
    }
}
