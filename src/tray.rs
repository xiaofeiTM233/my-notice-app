use crate::app;
use crate::icon;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// 构建系统托盘图标与右键菜单，并挂接全局事件处理器。
/// 返回 None 表示创建失败（此时应用仍可在通知中心内正常使用）。
pub fn build() -> Option<TrayIcon> {
    let menu = Menu::new();
    let show = MenuItem::with_id("show", "显示 / 隐藏通知中心", true, None);
    let pause = MenuItem::with_id("pause", "暂停弹窗", true, None);
    let sep = PredefinedMenuItem::separator();
    let quit = MenuItem::with_id("quit", "退出 NotifyHub", true, None);
    if menu.append_items(&[&show, &pause, &sep, &quit]).is_err() {
        return None;
    }

    let icon = Icon::from_rgba(icon::rgba(32, false), 32, 32).ok()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("NotifyHub")
        .with_icon(icon)
        .with_menu_on_left_click(false)
        .build()
        .ok()?;

    // 菜单项点击：按 id 分发到 UI 线程处理
    MenuEvent::set_event_handler(Some(|e: MenuEvent| match e.id().as_ref() {
        "show" => app::post(|a| a.toggle_center()),
        "pause" => app::post(|a| a.toggle_pause()),
        "quit" => app::post(|a| a.quit()),
        _ => {}
    }));

    // 左键单击切换通知中心；右键由系统弹出菜单
    TrayIconEvent::set_event_handler(Some(|e: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = e
        {
            app::post(|a| a.toggle_center());
        }
    }));

    Some(tray)
}
