//! my-notice-app：基于 GPUI 与 gpui-component 的桌面通知盒。
//!
//! 通过 SSE 实时接收服务端推送的通知，展示为可筛选、可标记已读/删除的
//! 通知卡片列表，并在窗口右上角弹出轻提示动画。

pub mod app;
pub mod config;
pub mod models;
pub mod sse;
pub mod ui;

pub use app::NotificationApp;
pub use config::Config;
