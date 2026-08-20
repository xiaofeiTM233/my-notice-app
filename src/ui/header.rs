//! 窗口标题条与工具栏：连接状态徽标、筛选、未读计数与快捷操作。

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable as _, StyledExt, ThemeMode, TitleBar, button::Button,
    button::ButtonVariants as _, h_flex,
};

use crate::app::{Filter, NotificationApp};
use crate::sse::ConnectionState;

/// 标题条：应用名 + 连接状态徽标。
pub fn render_header(app: &NotificationApp, cx: &Context<NotificationApp>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let (color, label) = connection_status(&app.connection_state, &theme);

    TitleBar::new().child(
        h_flex().gap_3().px_2().child(
            h_flex()
                .gap_2()
                .items_center()
                .child(div().text_sm().font_semibold().child("通知盒"))
                .child(
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .child(div().size_2().rounded_full().bg(color))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(label),
                        ),
                ),
        ),
    )
}

/// 工具栏：筛选 + 未读计数 + 主题切换 + 发送测试 + 清空。
pub fn render_toolbar(
    app: &NotificationApp,
    cx: &mut Context<NotificationApp>,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    let unread = app.unread_count();

    h_flex()
        .px_3()
        .py_2()
        .gap_3()
        .items_center()
        .border_b_1()
        .border_color(theme.border)
        .child(filter_segment(app, cx))
        .child(div().flex_1())
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(if unread == 0 {
                    "全部已读".to_string()
                } else {
                    format!("{unread} 条未读")
                }),
        )
        .child(theme_toggle_button(app, cx))
        .child(send_test_button(cx))
        .child(clear_button(cx))
}

/// 根据连接状态返回（圆点颜色，短文案）。
fn connection_status(
    state: &ConnectionState,
    theme: &gpui_component::Theme,
) -> (gpui::Hsla, String) {
    match state {
        ConnectionState::Connected => (theme.success, "已连接".into()),
        ConnectionState::Connecting => (theme.warning, "连接中…".into()),
        ConnectionState::Reconnecting { .. } => (theme.warning, "重连中".into()),
        ConnectionState::Error(_) => (theme.danger, "连接异常".into()),
        ConnectionState::Disconnected => (theme.muted, "未连接".into()),
    }
}

/// 通知筛选分段控件。
fn filter_segment(app: &NotificationApp, cx: &mut Context<NotificationApp>) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        .child(filter_pill(
            Filter::All,
            app.filter == Filter::All,
            "全部",
            cx,
        ))
        .child(filter_pill(
            Filter::Unread,
            app.filter == Filter::Unread,
            "未读",
            cx,
        ))
        .child(filter_pill(
            Filter::Read,
            app.filter == Filter::Read,
            "已读",
            cx,
        ))
}

/// 单个筛选按钮（选中态高亮）。
fn filter_pill(
    filter: Filter,
    active: bool,
    label: &'static str,
    cx: &mut Context<NotificationApp>,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    let (bg, fg, hover_bg) = if active {
        (theme.primary, theme.primary_foreground, theme.primary)
    } else {
        (
            gpui::transparent_black(),
            theme.muted_foreground,
            theme.list_hover,
        )
    };

    div()
        .id(label)
        .cursor_pointer()
        .px_2()
        .py_0p5()
        .rounded_md()
        .text_xs()
        .text_color(fg)
        .bg(bg)
        .hover(move |style| style.bg(hover_bg))
        .on_click(cx.listener(move |app, _ev, _window, cx| {
            app.filter = filter;
            cx.notify();
        }))
        .child(label)
}

/// 主题切换按钮（浅色/深色）。
fn theme_toggle_button(
    app: &NotificationApp,
    cx: &mut Context<NotificationApp>,
) -> impl IntoElement {
    let is_dark = matches!(app.theme_mode, ThemeMode::Dark);
    Button::new("toggle-theme")
        .icon(if is_dark {
            IconName::Sun
        } else {
            IconName::Moon
        })
        .ghost()
        .xsmall()
        .tooltip(if is_dark {
            "切换到浅色"
        } else {
            "切换到深色"
        })
        .on_click(cx.listener(|app, _ev, _window, cx| app.toggle_theme(cx)))
}

/// 发送测试通知按钮：请求服务端 `/send` 推送一条测试通知。
fn send_test_button(cx: &mut Context<NotificationApp>) -> impl IntoElement {
    Button::new("send-test")
        .label("发送测试")
        .icon(IconName::Bell)
        .ghost()
        .xsmall()
        .tooltip("请求服务端推送一条测试通知")
        .on_click(cx.listener(|app, _ev, _window, cx| {
            app.send_test();
            cx.notify();
        }))
}

/// 清空全部通知按钮。
fn clear_button(cx: &mut Context<NotificationApp>) -> impl IntoElement {
    Button::new("clear-all")
        .label("清空")
        .icon(IconName::Delete)
        .ghost()
        .xsmall()
        .tooltip("清空全部通知")
        .on_click(cx.listener(|app, _ev, _window, cx| {
            app.clear_all();
            cx.notify();
        }))
}
