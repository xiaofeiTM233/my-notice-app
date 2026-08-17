//! my-notice-app 入口。

mod app;
mod state;
mod toast;
#[cfg(target_os = "windows")]
mod tray;
mod ui;

use app::launch;

fn main() {
    if let Err(err) = launch() {
        eprintln!("[my-notice-app] 启动失败: {err:#}");
        std::process::exit(1);
    }
}
