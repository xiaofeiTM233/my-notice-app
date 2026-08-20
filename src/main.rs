//! my-notice-app 入口：初始化 GPUI 应用与主窗口。

use gpui::{App, Application, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_component::{Root, TitleBar};
use gpui_component_assets::Assets;
use my_notice_app::{Config, NotificationApp};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    Application::new().with_assets(Assets).run(|cx: &mut App| {
        gpui_component::init(cx);

        let config = Config::from_env();
        log::info!("服务端地址: {}", config.server_url);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(420.), px(560.)), cx)),
            titlebar: Some(TitleBar::title_bar_options()),
            app_id: Some("my-notice-app".to_string()),
            window_min_size: Some(size(px(360.), px(420.))),
            ..Default::default()
        };

        let result = cx.open_window(options, |window, cx| {
            let window_handle = window.window_handle();
            let view = cx.new(|cx| NotificationApp::new(config, window_handle, cx));
            cx.new(|cx| Root::new(view, window, cx))
        });

        match result {
            Ok(_window) => cx.activate(true),
            Err(err) => {
                eprintln!("打开窗口失败: {err:#}");
                std::process::exit(1);
            }
        }
    });
}
