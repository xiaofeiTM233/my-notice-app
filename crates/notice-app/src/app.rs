//! 应用装配：命令行参数、GPUI 初始化、主窗口创建。

use gpui::{App, Bounds, Size, WindowBounds, WindowKind, WindowOptions, px, size};
use gpui_component::{Root, TitleBar};
use gpui_component_assets::Assets;

use crate::state::ServerConfig;
use crate::ui::root::NoticeRoot;

/// 启动应用（阻塞运行 GUI 事件循环）。
pub fn launch() -> anyhow::Result<()> {
    let server = ServerConfig::from_args();

    let app = gpui_platform::application().with_assets(Assets);
    app.run(move |cx| {
        // 必须在任何 gpui-component 组件使用之前调用
        gpui_component::init(cx);
        cx.activate(true);

        open_main_window(server.clone(), cx);
    });

    Ok(())
}

/// 打开主窗口。
fn open_main_window(server: ServerConfig, cx: &mut App) {
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
            ..TitleBar::window_options()
        };

        let window = cx
            .open_window(options, |window, cx| {
                let root = cx.new(|cx| NoticeRoot::new(server.clone(), window, cx));
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
