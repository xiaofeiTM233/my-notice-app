//! SSE 客户端连接模块。
//!
//! GPUI 的异步执行器基于 smol，而 `reqwest` 依赖 tokio 运行时。为避免运行时
//! 冲突，本模块把 SSE 连接放到一个独立的 tokio 工作线程中执行，通过
//! `smol::channel` 把事件回传给 UI 线程。GPUI 线程只负责消费 `Receiver`。
//!
//! 特性：
//! - 自动重连：指数退避 + 随机抖动，重置后归零；
//! - 心跳超时：45 秒无数据判定连接僵死并强制重连；
//! - 优雅退出：`CancellationToken` + 线程 join；
//! - 断线期间保留所有历史通知，UI 侧可通过状态提示。

use std::{sync::Arc, thread::JoinHandle, time::Duration};

use anyhow::Context as _;
use futures_util::stream::StreamExt as _;
use reqwest::header::HeaderValue;
use smol::channel::{Receiver, Sender, bounded};
use tokio_util::sync::CancellationToken;
use url::Url;

use super::parser::{SseMessage, SseParser};

/// 连接状态机（UI 标题栏/状态栏展示）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// 已断开（未连接或已停止）。
    Disconnected,
    /// 正在建立连接。
    Connecting,
    /// 已连接，正在接收推送。
    Connected,
    /// 连接失败，等待退避重连。
    Reconnecting { attempt: u32 },
    /// 发生错误（伴随 `Reconnecting` 状态）。
    Error(String),
}

impl ConnectionState {
    pub fn label(&self) -> String {
        match self {
            Self::Disconnected => "未连接".into(),
            Self::Connecting => "连接中…".into(),
            Self::Connected => "已连接".into(),
            Self::Reconnecting { attempt } => format!("重连中 (第 {attempt} 次)"),
            Self::Error(msg) => format!("连接异常: {msg}"),
        }
    }
}

/// 从 SSE 客户端发往 UI 的事件。
#[derive(Debug, Clone)]
pub enum SseClientEvent {
    /// 连接状态变化。
    State(ConnectionState),
    /// 一条完整的 SSE 报文。
    Message(SseMessage),
}

/// SSE 客户端。
///
/// 用法：
/// ```no_run
/// use my_notice_app::sse::SseClient;
/// let (mut client, rx) = SseClient::new("http://127.0.0.1:8000".into());
/// client.connect();
/// ```
pub struct SseClient {
    server_url: String,
    events_tx: Sender<SseClientEvent>,
    cancel: CancellationToken,
    runtime: Arc<tokio::runtime::Runtime>,
    worker: Option<JoinHandle<()>>,
}

/// 连接/重连参数。
#[derive(Debug, Clone)]
pub struct SseConfig {
    /// 心跳超时：超过该时长没有收到任何数据则判定连接僵死。
    pub heartbeat_timeout: Duration,
    /// 初始退避间隔。
    pub backoff_base: Duration,
    /// 最大退避间隔。
    pub backoff_max: Duration,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout: Duration::from_secs(45),
            backoff_base: Duration::from_millis(500),
            backoff_max: Duration::from_secs(30),
        }
    }
}

impl SseClient {
    /// 创建一个 SSE 客户端，返回 `(客户端, 事件接收端)`。
    ///
    /// `server_url` 为服务端根地址，如 `http://127.0.0.1:8000`，
    /// SSE 端点固定追加为 `{server_url}/events`。
    pub fn new(server_url: String) -> (Self, Receiver<SseClientEvent>) {
        let (events_tx, events_rx) = bounded::<SseClientEvent>(128);
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("my-notice-sse")
                .enable_all()
                .build()
                .expect("failed to build tokio runtime"),
        );
        (
            Self {
                server_url,
                events_tx,
                cancel: CancellationToken::new(),
                runtime,
                worker: None,
            },
            events_rx,
        )
    }

    /// SSE 端点地址。
    fn sse_url(&self) -> Url {
        Url::parse(&format!("{}/events", self.server_url)).expect("invalid SSE endpoint")
    }

    /// 启动连接（幂等：已连接时忽略）。
    ///
    /// 内部自动处理断线重连，直到调用 [`Self::disconnect`]。
    pub fn connect(&mut self) {
        if self.worker.is_some() {
            return;
        }

        let tx = self.events_tx.clone();
        let cancel = CancellationToken::new();
        self.cancel = cancel.clone();
        let runtime = self.runtime.clone();
        let url = self.sse_url();
        let config = SseConfig::default();

        self.worker = Some(std::thread::spawn(move || {
            runtime.block_on(sse_loop(url, tx, cancel, config));
        }));
    }

    /// 停止连接并等待工作线程退出。
    pub fn disconnect(&mut self) {
        self.cancel.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = self
            .events_tx
            .send_blocking(SseClientEvent::State(ConnectionState::Disconnected));
    }

    /// 立即重连：取消当前工作线程后重新启动。
    pub fn reconnect(&mut self) {
        self.disconnect();
        self.connect();
    }

    /// 是否正在连接中。
    pub fn is_connected(&self) -> bool {
        self.worker.is_some()
    }

    /// 请求测试服务端推送一条通知（阻塞调用方，直到请求完成）。
    ///
    /// 用于「发送测试通知」按钮：通过独立运行时向 `{server_url}/send` 发起 POST。
    pub fn send_test_notification(&self, payload: serde_json::Value) -> anyhow::Result<()> {
        let url = format!("{}/send", self.server_url);
        let client = self.runtime.handle().spawn(async move {
            let http = reqwest::Client::new();
            http.post(&url).json(&payload).send().await
        });
        let resp = self
            .runtime
            .block_on(client)
            .context("failed to await test request")?;
        let resp = resp.context("test notification request failed")?;
        if !resp.status().is_success() {
            anyhow::bail!("server returned {}", resp.status());
        }
        Ok(())
    }
}

impl Drop for SseClient {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// 独立的 SSE 事件循环：连接 -> 消费 -> 断线重连（指数退避）。
async fn sse_loop(
    url: Url,
    tx: Sender<SseClientEvent>,
    cancel: CancellationToken,
    config: SseConfig,
) {
    let mut attempt: u32 = 0;

    loop {
        if cancel.is_cancelled() {
            return;
        }

        if attempt > 0 {
            let backoff = backoff_delay(attempt, &config);
            let _ = tx.send_blocking(SseClientEvent::State(ConnectionState::Reconnecting {
                attempt,
            }));
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = tokio::time::sleep(backoff) => {}
            }
        }

        attempt += 1;
        let _ = tx.send_blocking(SseClientEvent::State(ConnectionState::Connecting));

        match connect_once(&url, &tx, &cancel, &config).await {
            Ok(graceful) => {
                if cancel.is_cancelled() {
                    return;
                }
                if graceful {
                    // 服务端主动关闭连接：快速重连（不按失败退避，但限制最低间隔）。
                    attempt = attempt.saturating_sub(1).max(1);
                }
            }
            Err(err) => {
                let _ = tx.send_blocking(SseClientEvent::State(ConnectionState::Error(
                    err.to_string(),
                )));
            }
        }

        let _ = tx.send_blocking(SseClientEvent::State(ConnectionState::Disconnected));
    }
}

/// 尝试建立一次连接并持续消费 SSE 流。
///
/// 返回 `Ok(true)` 表示流被服务端正常关闭；`Ok(false)` 表示被取消；
/// `Err` 表示连接或读取失败。
async fn connect_once(
    url: &Url,
    tx: &Sender<SseClientEvent>,
    cancel: &CancellationToken,
    config: &SseConfig,
) -> anyhow::Result<bool> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .user_agent("my-notice-app/0.1")
        .build()?;

    let response = client
        .get(url.clone())
        .header("Accept", HeaderValue::from_static("text/event-stream"))
        .header("Cache-Control", HeaderValue::from_static("no-cache"))
        .send()
        .await
        .context("failed to connect SSE endpoint")?;

    if !response.status().is_success() {
        anyhow::bail!("server returned HTTP {}", response.status());
    }

    let _ = tx.send_blocking(SseClientEvent::State(ConnectionState::Connected));

    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => return Ok(false),
            next = tokio::time::timeout(config.heartbeat_timeout, stream.next()) => {
                match next {
                    Err(_) => anyhow::bail!("connection idle (no data for {:?})", config.heartbeat_timeout),
                    Ok(None) => return Ok(true),
                    Ok(Some(Err(err))) => return Err(err.into()),
                    Ok(Some(Ok(bytes))) => {
                        parser.feed(&bytes);
                        for message in parser.drain() {
                            if tx.send_blocking(SseClientEvent::Message(message)).is_err() {
                                // 接收端已关闭（应用退出）。
                                return Ok(false);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 指数退避（带随机抖动，避免重连风暴）。
fn backoff_delay(attempt: u32, config: &SseConfig) -> Duration {
    let base_ms = config.backoff_base.as_millis().max(100) as u64;
    let exponent = attempt.saturating_sub(1).min(8);
    let raw = base_ms.saturating_mul(1u64 << exponent);
    let capped = raw.min(config.backoff_max.as_millis() as u64);
    let jitter = {
        // 简单伪随机，避免引入额外依赖。
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (
            attempt,
            std::time::SystemTime::now()
                .elapsed()
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        )
            .hash(&mut hasher);
        hasher.finish() % (capped / 4 + 1)
    };
    Duration::from_millis(capped + jitter)
}
