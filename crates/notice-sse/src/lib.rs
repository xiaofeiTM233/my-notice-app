//! # notice-sse
//!
//! SSE（Server-Sent Events）客户端连接模块。
//!
//! 职责：
//! - 建立并维持到通知服务端 `/events` 的 SSE 长连接；
//! - 按 SSE 规范逐行解析事件流（`event:` / `data:` / `id:` / `retry:` / 注释心跳）；
//! - 将 `event: notice` 且 `data` 为 JSON 的消息反序列化为 [`notice_model::NoticeMessage`]；
//! - 断线自动重连（指数退避），并支持 `Last-Event-ID` 续传；
//! - 心跳超时保护：超过阈值未收到任何数据视为连接失效，主动断开重连。
//!
//! 线程模型：`SseClient::spawn` 会在独立线程中创建 tokio 运行时执行网络 IO，
//! 通过 `std::sync::mpsc` 通道把事件推送给调用方（GUI 主线程轮询即可），
//! 避免与 GPUI 自身的执行器相互干扰。

mod client;

pub use client::{SseClient, SseEvent, SseHandle};
