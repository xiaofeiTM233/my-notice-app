mod app;
mod config;
mod feed;
mod icon;
mod model;
mod store;
mod toast;
mod tray;
mod view;
mod win32;

slint::include_modules!();

fn main() {
    let (cfg, warn) = config::load();
    if let Some(w) = warn {
        eprintln!("[config] {w}");
    }

    let dir = config::dir();
    let store = store::Store::load(dir, cfg.store.max, cfg.store.persist);
    let feed_cfg = cfg.feed.clone();

    let app = match app::App::new(cfg, store) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("无法创建主窗口：{e}");
            std::process::exit(1);
        }
    };
    app::install(app.clone());

    // 系统托盘（失败不致命）
    if let Some(t) = tray::build() {
        *app.tray.borrow_mut() = Some(t);
    }

    // 初始化列表与托盘角标
    app.refresh();

    // 后台通知拉取线程（WebSocket / 长轮询，断线自动重连）
    let feed = feed::spawn(feed_cfg, |ev| app::post(move |a| a.on_ev(ev)));
    *app.feed.borrow_mut() = Some(feed);

    // 按配置决定是否启动时显示通知中心
    if !app.cfg.borrow().ui.start_hidden {
        app.show_center();
    }

    if let Err(e) = slint::run_event_loop_until_quit() {
        eprintln!("[slint] 事件循环异常退出：{e}");
    }

    // 收尾：停止拉取并落盘
    if let Some(f) = app.feed.borrow().as_ref() {
        f.stop();
    }
    app.flush();
    std::process::exit(0);
}
