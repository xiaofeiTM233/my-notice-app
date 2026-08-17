//! 应用装配：命令行参数、GPUI 初始化、主窗口创建。

use gpui::{App, AppContext, Bounds, Size, WindowBounds, WindowKind, WindowOptions, px, size};
use gpui_component::{Root, TitleBar};
use gpui_component_assets::Assets;

use crate::state::ServerConfig;
use crate::ui::root::NoticeRoot;

/// 系统级事件：由托盘 / toast 窗口发出，主窗口轮询处理。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {
    /// 显示 / 激活主窗口（托盘左键、菜单"显示"、toast 卡片点击）。
    ShowMainWindow,
    /// 退出应用（托盘菜单"退出"）。
    Quit,
    /// 将指定通知标记为已读（toast 空白区域点击）。
    MarkRead(String),
}

/// 启动应用（阻塞运行 GUI 事件循环）。
pub fn launch() -> anyhow::Result<()> {
    let server = ServerConfig::from_args();

    let app = gpui::Application::new().with_assets(Assets);
    app.run(move |cx| {
        // 必须在任何 gpui-component 组件使用之前调用
        gpui_component::init(cx);
        cx.activate(true);

        // 系统事件通道：托盘与 toast 窗口 → 主窗口
        let (system_tx, system_rx) = std::sync::mpsc::channel::<SystemEvent>();

        // 屏幕右下角 toast 窗口（全局句柄供主窗口推送）
        let toast_window = crate::toast::open_toast_window(cx, system_tx.clone());
        cx.set_global(crate::toast::ToastHub(toast_window));

        open_main_window(server.clone(), system_tx, system_rx, cx);
    });

    Ok(())
}

/// 打开主窗口。
fn open_main_window(
    server: ServerConfig,
    system_tx: std::sync::mpsc::Sender<SystemEvent>,
    system_rx: std::sync::mpsc::Receiver<SystemEvent>,
    cx: &mut App,
) {
    let window_bounds = Bounds::centered(None, size(px(1080.), px(720.)), cx);

    cx.spawn(async move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            window_min_size: Some(Size {
                width: px(760.),
                height: px(480.),
            }),
            kind: WindowKind::Normal,
            // 使用 gpui-component 的 TitleBar：窗口自带自定义标题栏
            titlebar: Some(TitleBar::title_bar_options()),
            ..Default::default()
        };

        let window = cx
            .open_window(options, |window, cx| {
                let root = cx.new(|cx| {
                    NoticeRoot::new(server.clone(), system_tx.clone(), system_rx, window, cx)
                });
                // 窗口第一层必须是 Root
                cx.new(|cx| Root::new(root, window, cx))
            })
            .expect("打开主窗口失败");

        window.update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title("My Notice — 通知盒");
        })?;

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}
