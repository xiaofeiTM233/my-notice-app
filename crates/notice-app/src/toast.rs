//! 屏幕右下角的类 toast 弹窗提醒。
//!
//! 实现方式：创建一个**独立的无边框、置顶、透明背景**小窗口
//!（`WindowKind::PopUp` 在 Windows 上即 `WS_EX_TOOLWINDOW` 无边框样式），
//! 固定在主显示器右下角；新通知到达时把卡片推入该窗口的栈中，
//! 5 秒后自动滑出消失。
//!
//! 点击交互（GPUI 事件层实现）：
//! - 点击卡片的**图标 / 文字内容区** → `SystemEvent::ShowMainWindow`（打开主界面）；
//! - 点击卡片的**空白区域**（内边距等）→ `SystemEvent::MarkRead`（标记已读）并关闭该卡片。
//!
//! 主窗口通过全局 [`ToastHub`] 句柄向 toast 窗口推送内容，
//! 因此即使主窗口被隐藏（驻留托盘），toast 依然会在屏幕右下角弹出。

use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, AppContext, Bounds, Context, Entity, InteractiveElement,
    IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, point, px, size,
};
use gpui_component::{ActiveTheme as _, Icon, StyledExt as _, h_flex, v_flex};
use notice_model::{NoticeKind, NoticeMessage};

use crate::app::SystemEvent;
use crate::ui::{kind_color, kind_icon};

/// toast 窗口向主窗口发送系统事件所用的发送端（由 `configure_toast_window` 注入）。
static TOAST_EVENT_TX: OnceLock<Mutex<Option<Sender<SystemEvent>>>> = OnceLock::new();

/// 发送一条系统事件（失败静默）。
fn send_event(event: SystemEvent) {
    let tx = TOAST_EVENT_TX
        .get()
        .and_then(|m| m.lock().ok())
        .and_then(|guard| guard.clone());
    if let Some(tx) = tx {
        let _ = tx.send(event);
    }
}

/// 单条 toast 内容。
#[derive(Debug, Clone)]
pub struct ToastItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub kind: NoticeKind,
    pub created_at: i64,
}

impl ToastItem {
    pub fn from_notice(message: &NoticeMessage, now_unix: i64) -> Self {
        Self {
            id: message.id.clone(),
            title: message.content.title.clone(),
            summary: message
                .summary_or_body()
                .map(|s| s.chars().take(80).collect::<String>())
                .unwrap_or_default(),
            kind: message.meta.kind,
            created_at: now_unix,
        }
    }
}

/// 全局句柄：主窗口据此向 toast 窗口推送内容。
#[derive(Clone)]
pub struct ToastHub(pub Entity<ToastWindow>);

impl gpui::Global for ToastHub {}

/// toast 显示时长（秒）。
const TOAST_TTL_SECS: i64 = 5;
/// 同时显示的最大 toast 数。
const MAX_TOASTS: usize = 4;
/// toast 窗口尺寸。
const TOAST_W: f32 = 380.;
const TOAST_H: f32 = 320.;

/// toast 窗口视图。
pub struct ToastWindow {
    toasts: Vec<ToastItem>,
}

impl ToastWindow {
    /// 推入一条 toast 并安排 5 秒后自动移除。
    pub fn push(&mut self, item: ToastItem, cx: &mut Context<Self>) {
        self.toasts.push(item);
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }
        cx.notify();

        // 到期自动移除
        let view = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(TOAST_TTL_SECS as u64))
                .await;
            let _ = view.update(cx, |this, cx| {
                let now = chrono::Utc::now().timestamp();
                let before = this.toasts.len();
                this.toasts
                    .retain(|t| now - t.created_at < TOAST_TTL_SECS);
                if this.toasts.len() != before {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn remove(&mut self, id: &str, cx: &mut Context<Self>) {
        let before = self.toasts.len();
        self.toasts.retain(|t| t.id != id);
        if self.toasts.len() != before {
            cx.notify();
        }
    }
}

impl Render for ToastWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 透明窗口：外层不设背景，仅渲染卡片栈（底部对齐）。
        // 先收集为 AnyElement，避免闭包捕获 cx 逃逸。
        let cards: Vec<gpui::AnyElement> = self
            .toasts
            .iter()
            .cloned()
            .map(|toast| toast_card(toast, cx).into_any_element())
            .collect();

        v_flex()
            .size_full()
            .justify_end()
            .gap_2()
            .p_2()
            .children(cards)
    }
}

/// 单张 toast 卡片，带滑入动画。
///
/// 交互：
/// - 卡片根元素（空白区域 / 内边距）点击 → 标记已读 + 关闭；
/// - 内容区（图标 + 文字）点击 → 打开主界面（阻止冒泡）。
fn toast_card(toast: ToastItem, cx: &mut Context<ToastWindow>) -> impl IntoElement {
    let color = kind_color(cx, toast.kind);
    let id = toast.id.clone();
    let anim_id = hash_id(&toast.id);

    let animation = Animation::new(Duration::from_millis(280))
        .with_easing(gpui_component::animation::cubic_bezier(0.25, 0.1, 0.25, 1.));

    h_flex()
        .id(gpui::ElementId::NamedInteger("toast".into(), anim_id))
        .w_full()
        .p_3()
        .rounded(px(10.))
        .bg(cx.theme().popover)
        .shadow_lg()
        // 卡片空白区域：标记已读并关闭
        .on_click(cx.listener(move |this, _, _, cx| {
            send_event(SystemEvent::MarkRead(id.clone()));
            this.remove(&id, cx);
        }))
        .child(
            // 内容区：图标 + 文字，点击打开主界面（阻止冒泡到卡片根）
            h_flex()
                .id(gpui::ElementId::NamedInteger("toast-content".into(), anim_id))
                .flex_1()
                .items_center()
                .gap_2()
                .on_click(cx.listener(|_, _, _, cx| {
                    send_event(SystemEvent::ShowMainWindow);
                    cx.stop_propagation();
                }))
                .child(
                    div()
                        .flex()
                        .size(px(30.))
                        .flex_shrink_0()
                        .rounded(px(8.))
                        .items_center()
                        .justify_center()
                        .bg(color.opacity(0.14))
                        .child(Icon::new(kind_icon(toast.kind)).size(px(16.)).text_color(color)),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .overflow_hidden()
                        .gap_1()
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_sm()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child(toast.title.clone()),
                        )
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(toast.summary.clone()),
                        ),
                ),
        )
        .with_animation(
            gpui::ElementId::NamedInteger("toast-in".into(), anim_id),
            animation,
            move |this, delta| {
                this.opacity(delta)
                    .top(px((1.0 - delta) * 24.))
            },
        )
}

/// 简单字符串哈希（用于动画元素 ID）。
fn hash_id(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// 创建 toast 窗口（屏幕右下角，无边框置顶，不抢焦点）。
///
/// 返回 toast 窗口的 [`Entity`] 句柄，供主窗口推送内容。
pub fn open_toast_window(cx: &mut App, event_tx: Sender<SystemEvent>) -> Entity<ToastWindow> {
    let origin = if let Some(display) = cx.primary_display() {
        let b = display.bounds();
        // 底部预留任务栏空间
        point(
            b.origin.x + b.size.width - px(TOAST_W) - px(16.),
            b.origin.y + b.size.height - px(TOAST_H) - px(56.),
        )
    } else {
        point(px(600.), px(400.))
    };

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            origin,
            size(px(TOAST_W), px(TOAST_H)),
        ))),
        kind: WindowKind::PopUp,
        focus: false,
        show: true,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };

    // open_window 返回 WindowHandle，这里通过共享槽把内部创建的 Entity 带出来
    let slot: Rc<std::cell::Cell<Option<Entity<ToastWindow>>>> = Rc::new(std::cell::Cell::new(None));
    let slot_out = slot.clone();
    cx.open_window(options, move |window, cx| {
        // Windows：强制无边框 + 置顶（见 configure_toast_window）
        #[cfg(target_os = "windows")]
        configure_toast_window(window, event_tx);
        let entity = cx.new(|_| ToastWindow { toasts: vec![] });
        slot_out.set(Some(entity.clone()));
        entity
    })
    .expect("创建 toast 窗口失败");

    slot.take().expect("toast 窗口 Entity 未创建")
}

/// 把 toast 窗口配置为：无边框、置顶（Windows）。
///
/// gpui 的 `WindowKind::PopUp` 与透明背景在 Windows 上的实际样式不完全可控，
/// 这里在窗口创建后用 Win32 强制覆盖：
/// - 清除标题栏 / 边框 / 系统菜单样式；
/// - 追加 `WS_EX_LAYERED | WS_EX_TOOLWINDOW`；
/// - `SetWindowPos(HWND_TOPMOST)` 强制置顶（全屏应用之上也能显示）。
///
/// 鼠标事件由 GPUI 层正常处理（卡片内容区打开主界面 / 空白区标记已读）。
#[cfg(target_os = "windows")]
fn configure_toast_window(window: &mut Window, event_tx: Sender<SystemEvent>) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST, SetWindowLongPtrW, SetWindowPos,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_CAPTION, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
        WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
    };

    unsafe {
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(h) = handle.as_raw() else {
            return;
        };
        let hwnd = HWND(h.hwnd.get() as *mut std::ffi::c_void);

        // 1) 清除边框 / 标题栏 / 系统菜单 / 最大化最小化
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let mask = !(WS_CAPTION.0
            | WS_THICKFRAME.0
            | WS_SYSMENU.0
            | WS_MAXIMIZEBOX.0
            | WS_MINIMIZEBOX.0) as u32;
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, style & (mask as i32 as isize));

        // 2) 追加分层与工具窗口样式
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let ex_add = (WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0) as i32 as isize;
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | ex_add);

        // 3) 强制置顶
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );

        // 4) 注入系统事件发送端（供 GPUI 卡片点击回调使用）
        let _ = TOAST_EVENT_TX.get_or_init(|| Mutex::new(None));
        if let Some(m) = TOAST_EVENT_TX.get() {
            if let Ok(mut guard) = m.lock() {
                *guard = Some(event_tx);
            }
        }
    }
}
