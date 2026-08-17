//! 详情面板：展示选中通知的完整内容（标题、元信息、正文、浏览记录等）。

use std::rc::Rc;

use gpui::{
    App, Div, ParentElement, SharedString, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _, h_flex, text, v_flex,
};
use notice_model::NoticeMessage;

use crate::ui::{format_full_time, kind_color, kind_icon, priority_color};

/// 详情面板主体（不含外层滚动容器，由调用方控制尺寸）。
///
/// 布局：左侧为类型图标独立列，右侧为标题与全部内容列。
pub fn detail_content(message: &Rc<NoticeMessage>, window: &mut Window, cx: &mut App) -> Div {
    let kind = message.meta.kind;
    let kind_color = kind_color(cx, kind);
    let priority = message.meta.priority;
    let priority_color = priority_color(cx, priority);

    h_flex()
        .size_full()
        .gap_3()
        .items_start()
        .child(
            // 左侧：类型图标独立列；div 需显式 flex 才能居中子元素
            div()
                .flex()
                .size(px(40.))
                .flex_shrink_0()
                .rounded(cx.theme().radius)
                .items_center()
                .justify_center()
                .bg(kind_color.opacity(0.14))
                .child(Icon::new(kind_icon(kind)).large().text_color(kind_color)),
        )
        .child(
            // 右侧：标题与内容列
            v_flex()
                .flex_1()
                .h_full()
                .gap_3()
                .child(
                    div()
                        .text_lg()
                        .font_semibold()
                        .text_color(cx.theme().foreground)
                        .child(message.content.title.clone()),
                )
                .child(
                    // 元信息行：类型 / 优先级 / 频道 / 时间
                    h_flex()
                        .items_center()
                        .gap_1()
                        .flex_wrap()
                        .child(meta_pill(kind.display_name(), kind_color))
                        .child(meta_pill(priority.display_name(), priority_color))
                        .child(meta_pill(
                            &message.meta.channel,
                            cx.theme().muted_foreground,
                        ))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format_full_time(message.meta.timestamp)),
                        ),
                )
                .when_some(message.content.author.clone(), |this, author| {
                    let name = author.name.unwrap_or_else(|| "未知用户".to_string());
                    this.child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .size(px(24.))
                                    .rounded(px(12.))
                                    .items_center()
                                    .justify_center()
                                    .bg(cx.theme().list_hover)
                                    .child(Icon::new(IconName::User).xsmall()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().foreground)
                                    .child(name),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{} 人浏览", message.interaction.views.len())),
                            ),
                    )
                })
                .when(!message.content.tags.is_empty(), |this| {
                    this.child(h_flex().items_center().gap_1().flex_wrap().children(
                        message.content.tags.iter().map(|tag| {
                            div()
                                .px_1()
                                .py_0p5()
                                .rounded(cx.theme().radius)
                                .bg(cx.theme().list_hover)
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(tag.clone())
                        }),
                    ))
                })
                .child(div().w_full().h(px(1.)).bg(cx.theme().border).my_1())
                .child(
                    // 正文：支持 Markdown / HTML / 纯文本
                    div().flex_1().overflow_hidden().child(
                        text::TextView::markdown(
                            "detail-body",
                            body_text(message).to_string(),
                            window,
                            cx,
                        )
                        .scrollable(true)
                        .selectable(true)
                        .text_base(),
                    ),
                )
                .when_some(latest_view(message), |this, view_text| {
                    this.child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(Icon::new(IconName::Eye).xsmall())
                            .child(div().child(view_text)),
                    )
                }),
        )
}

/// 无选中时的占位内容。
pub fn empty_placeholder(cx: &App) -> Div {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_2()
        .text_color(cx.theme().muted_foreground.opacity(0.6))
        .child(Icon::new(IconName::Inbox).size_12())
        .child(div().text_sm().child("选择左侧一条通知查看详情"))
}

/// 小圆角标签（类型 / 优先级 / 频道）。
fn meta_pill(text: &str, color: gpui::Hsla) -> Div {
    div()
        .px_1()
        .py_0p5()
        .rounded(px(10.))
        .bg(color.opacity(0.14))
        .text_xs()
        .text_color(color)
        .child(SharedString::from(text.to_string()))
}

/// 正文：优先 body，回退 summary。
fn body_text(message: &NoticeMessage) -> &str {
    message
        .content
        .body
        .as_deref()
        .or(message.content.summary.as_deref())
        .unwrap_or("（本条通知没有正文内容）")
}

/// 最近一条浏览记录描述。
fn latest_view(message: &NoticeMessage) -> Option<String> {
    let view = message.interaction.views.last()?;
    let when = format_full_time(view.timestamp);
    match view.body.as_deref() {
        Some(body) if !body.is_empty() => Some(format!("最近浏览（{when}）：{body}")),
        _ => Some(format!("最近浏览：{when}")),
    }
}
