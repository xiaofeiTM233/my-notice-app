//! SSE 客户端实现。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use futures_util::StreamExt;
use notice_model::NoticeMessage;

/// 连接过程中上报给 GUI 层的事件。
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// 已建立连接。
    Connected { url: String },
    /// 连接断开（服务端关闭 / 网络异常）。
    Disconnected { reason: String },
    /// 即将进行第 `attempt` 次重连，`next_attempt_in` 为等待时长。
    Reconnecting {
        attempt: u32,
        next_attempt_in: Duration,
    },
    /// 收到心跳注释（`: ping` 等），说明连接仍然存活。
    Ping,
    /// 收到一条通知消息（`event: notice` + JSON）。
    Notice(NoticeMessage),
    /// 收到非 notice 事件（原样透传，便于调试与扩展）。
    Raw {
        event: Option<String>,
        id: Option<String>,
        data: String,
    },
    /// 发生错误（HTTP 错误、数据解析失败等）。
    Error { message: String },
}

/// SSE 客户端配置。
#[derive(Debug, Clone)]
pub struct SseClient {
    /// 服务端地址，例如 `http://127.0.0.1:8866/events`。
    url: String,
    /// 附加请求头。
    headers: Vec<(String, String)>,
    /// 首次重连等待（指数退避基准）。
    initial_backoff: Duration,
    /// 重连等待上限。
    max_backoff: Duration,
    /// 心跳超时：超过该时长未收到任何字节则判定连接失效。
    heartbeat_timeout: Duration,
}

/// 一个已停止/已断开的连接结果。
#[derive(Debug)]
enum ConnectEnd {
    /// 收到停止信号，主动退出。
    Stopped,
    /// 连接正常结束（服务端关闭）。
    Ended(String),
}

impl SseClient {
    /// 创建一个 SSE 客户端。
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(45),
        }
    }

    /// 附加自定义请求头（如鉴权 Token）。
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// 设置指数退避参数。
    pub fn with_backoff(mut self, initial: Duration, max: Duration) -> Self {
        self.initial_backoff = initial;
        self.max_backoff = max;
        self
    }

    /// 设置心跳超时。
    pub fn with_heartbeat_timeout(mut self, timeout: Duration) -> Self {
        self.heartbeat_timeout = timeout;
        self
    }

    /// 启动连接。
    ///
    /// 返回句柄（用于停止）与事件接收端。连接循环运行在独立线程的 tokio
    /// 运行时中，调用方需要自行轮询接收端。
    pub fn spawn(self) -> (SseHandle, Receiver<SseEvent>) {
        let (tx, rx) = mpsc::channel::<SseEvent>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let client = self;

        std::thread::Builder::new()
            .name("notice-sse".to_string())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(err) => {
                        let _ = tx.send(SseEvent::Error {
                            message: format!("创建 tokio 运行时失败: {err}"),
                        });
                        return;
                    }
                };
                runtime.block_on(async move {
                    client.run_loop(&tx, &stop_flag).await;
                });
            })
            .expect("无法启动 SSE 线程");

        (SseHandle { stop }, rx)
    }

    /// 主循环：连接 → 消费 → 断开 → 退避重连。
    async fn run_loop(&self, tx: &Sender<SseEvent>, stop: &Arc<AtomicBool>) {
        let mut attempt: u32 = 0;
        let mut last_event_id: Option<String> = None;

        loop {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            match self.connect_once(tx, stop, &mut last_event_id).await {
                Ok(ConnectEnd::Stopped) => return,
                Ok(ConnectEnd::Ended(reason)) => {
                    if !emit(tx, SseEvent::Disconnected { reason }, stop) {
                        return;
                    }
                }
                Err(err) => {
                    if !emit(
                        tx,
                        SseEvent::Error {
                            message: format!("{err:#}"),
                        },
                        stop,
                    ) {
                        return;
                    }
                }
            }

            if stop.load(Ordering::Relaxed) {
                return;
            }
            attempt = attempt.saturating_add(1);
            let delay = self.backoff_for(attempt);
            if !emit(
                tx,
                SseEvent::Reconnecting {
                    attempt,
                    next_attempt_in: delay,
                },
                stop,
            ) {
                return;
            }
            if !sleep_interruptible(delay, stop).await {
                return;
            }
        }
    }

    /// 单次连接：发送请求、解析事件流，直到断开或停止。
    async fn connect_once(
        &self,
        tx: &Sender<SseEvent>,
        stop: &Arc<AtomicBool>,
        last_event_id: &mut Option<String>,
    ) -> Result<ConnectEnd> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("创建 HTTP 客户端失败")?;

        let mut request = http
            .get(&self.url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header(
                "User-Agent",
                concat!("my-notice-app/", env!("CARGO_PKG_VERSION")),
            );
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        if let Some(id) = last_event_id.as_deref() {
            request = request.header("Last-Event-ID", id);
        }

        let response = request.send().await.context("连接通知服务端失败")?;
        if !response.status().is_success() {
            bail!("服务端返回错误状态码: HTTP {}", response.status());
        }

        if !emit(
            tx,
            SseEvent::Connected {
                url: self.url.clone(),
            },
            stop,
        ) {
            return Ok(ConnectEnd::Stopped);
        }

        let mut stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();

        // SSE 当前事件的状态
        let mut data_lines: Vec<String> = Vec::new();
        let mut event: Option<String> = None;
        let mut id: Option<String> = None;

        loop {
            // 心跳保护：整个“读取一行”动作必须在超时内完成
            let line = match tokio::time::timeout(
                self.heartbeat_timeout,
                read_line(&mut stream, &mut buffer),
            )
            .await
            {
                Ok(Ok(line)) => line,
                Ok(Err(err)) => {
                    if err.downcast_ref::<reqwest::Error>().is_some() {
                        return Err(err.context("读取事件流失败"));
                    }
                    return Err(err);
                }
                Err(_) => {
                    bail!(
                        "心跳超时：{}s 内未收到任何数据，主动断开重连",
                        self.heartbeat_timeout.as_secs()
                    )
                }
            };

            if stop.load(Ordering::Relaxed) {
                return Ok(ConnectEnd::Stopped);
            }

            match line {
                None => {
                    // 流结束
                    return Ok(ConnectEnd::Ended("服务端关闭了连接".into()));
                }
                Some(line) => {
                    if line.is_empty() {
                        // 空行 = 事件定界符，派发当前累积的事件
                        dispatch_event(
                            tx,
                            stop,
                            &mut data_lines,
                            &mut event,
                            &mut id,
                            last_event_id,
                        );
                    } else if line[0] == b':' {
                        // 注释行即心跳
                        if !emit(tx, SseEvent::Ping, stop) {
                            return Ok(ConnectEnd::Stopped);
                        }
                    } else {
                        parse_field(&line, &mut data_lines, &mut event, &mut id);
                    }
                }
            }
        }
    }

    /// 指数退避：`initial * 2^(attempt-1)`，上限 `max`。
    fn backoff_for(&self, attempt: u32) -> Duration {
        let base_ms = self.initial_backoff.as_millis().max(100) as u64;
        let shift = attempt.saturating_sub(1).min(16);
        let ms = base_ms.saturating_mul(1u64 << shift);
        let ms = ms.min(self.max_backoff.as_millis().max(100) as u64);
        Duration::from_millis(ms)
    }
}

/// 停止句柄：`stop()` 后连接线程会在下一个安全点退出。
#[derive(Debug)]
pub struct SseHandle {
    stop: Arc<AtomicBool>,
}

impl SseHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// 发送事件；接收端已关闭（如窗口退出）时置停止标志并返回 false。
fn emit(tx: &Sender<SseEvent>, event: SseEvent, stop: &Arc<AtomicBool>) -> bool {
    match tx.send(event) {
        Ok(()) => true,
        Err(_) => {
            stop.store(true, Ordering::Relaxed);
            false
        }
    }
}

/// 可被停止信号打断的睡眠。
async fn sleep_interruptible(duration: Duration, stop: &Arc<AtomicBool>) -> bool {
    let mut remaining = duration;
    const STEP: Duration = Duration::from_millis(200);
    while !remaining.is_zero() {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        let step = remaining.min(STEP);
        tokio::time::sleep(step).await;
        remaining = remaining.saturating_sub(step);
    }
    true
}

/// 从字节流中读取一行（去掉 `\n` 与末尾 `\r`）。
///
/// 返回 `None` 表示流已结束。跨 chunk 边界自动缓冲，流结束时冲刷残留内容。
async fn read_line(
    stream: &mut (impl futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin),
    buffer: &mut Vec<u8>,
) -> Result<Option<Vec<u8>>> {
    loop {
        if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = buffer.drain(..=pos).collect();
            line.pop(); // 去掉 \n
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
        match stream.next().await {
            Some(Ok(chunk)) => buffer.extend_from_slice(&chunk),
            Some(Err(err)) => return Err(anyhow!(err)),
            None => {
                if buffer.is_empty() {
                    return Ok(None);
                }
                let mut tail = std::mem::take(buffer);
                if tail.last() == Some(&b'\r') {
                    tail.pop();
                }
                return Ok(Some(tail));
            }
        }
    }
}

/// 解析一行 SSE 字段：`field: value`。
fn parse_field(
    line: &[u8],
    data_lines: &mut Vec<String>,
    event: &mut Option<String>,
    id: &mut Option<String>,
) {
    let text = String::from_utf8_lossy(line);
    let (field, value) = match text.split_once(':') {
        Some((f, v)) => (f.trim(), v.strip_prefix(' ').unwrap_or(v).trim_end()),
        None => (text.as_ref().trim(), ""),
    };
    match field {
        "data" => data_lines.push(value.to_string()),
        "event" => *event = Some(value.to_string()),
        "id" => {
            // 按规范：含 \0 的 id 应忽略
            if !value.contains('\0') {
                *id = Some(value.to_string());
            }
        }
        // "retry" 用于动态调整重连间隔，这里忽略（使用客户端配置）
        _ => {}
    }
}

/// 空行定界时派发事件。
#[allow(clippy::too_many_arguments)]
fn dispatch_event(
    tx: &Sender<SseEvent>,
    stop: &Arc<AtomicBool>,
    data_lines: &mut Vec<String>,
    event: &mut Option<String>,
    id: &mut Option<String>,
    last_event_id: &mut Option<String>,
) {
    let event_name = event.clone();
    let event_id = id.clone();
    let payload = data_lines.join("\n");

    // 先重置状态，再派发（避免派发过程中出错影响后续解析）
    data_lines.clear();
    *event = None;
    *id = None;

    if payload.is_empty() {
        return;
    }
    if let Some(ev) = &event_id {
        *last_event_id = Some(ev.clone());
    }

    match event_name.as_deref() {
        Some("notice") => match serde_json::from_str::<NoticeMessage>(&payload) {
            Ok(message) => {
                let _ = emit(tx, SseEvent::Notice(message), stop);
            }
            Err(err) => {
                let _ = emit(
                    tx,
                    SseEvent::Error {
                        message: format!("notice 数据解析失败: {err:#}"),
                    },
                    stop,
                );
                let _ = emit(
                    tx,
                    SseEvent::Raw {
                        event: event_name,
                        id: event_id,
                        data: payload,
                    },
                    stop,
                );
            }
        },
        Some("ping") | Some("hello") => {
            let _ = emit(tx, SseEvent::Ping, stop);
        }
        _ => {
            let _ = emit(
                tx,
                SseEvent::Raw {
                    event: event_name,
                    id: event_id,
                    data: payload,
                },
                stop,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sse_fields() {
        let mut data = Vec::new();
        let mut event = None;
        let mut id = None;

        parse_field(b"event: notice", &mut data, &mut event, &mut id);
        parse_field(b"id: abc-1", &mut data, &mut event, &mut id);
        parse_field(b"data: {\"id\":\"x\"}", &mut data, &mut event, &mut id);
        parse_field(b"data: tail", &mut data, &mut event, &mut id);

        assert_eq!(event.as_deref(), Some("notice"));
        assert_eq!(id.as_deref(), Some("abc-1"));
        assert_eq!(data, vec!["{\"id\":\"x\"}", "tail"]);
    }

    #[test]
    fn parses_field_without_value() {
        let mut data = Vec::new();
        let mut event = None;
        let mut id = None;
        parse_field(b"data:", &mut data, &mut event, &mut id);
        assert_eq!(data, vec![""]);
    }

    #[test]
    fn ignores_nul_in_event_id() {
        let mut data = Vec::new();
        let mut event = None;
        let mut id = Some("old".to_string());
        parse_field(b"id: bad\0id", &mut data, &mut event, &mut id);
        assert_eq!(id.as_deref(), Some("old"));
    }

    #[test]
    fn backoff_grows_exponentially_and_caps() {
        let client = SseClient::new("http://127.0.0.1:1/events")
            .with_backoff(Duration::from_secs(1), Duration::from_secs(30));
        assert_eq!(client.backoff_for(1), Duration::from_secs(1));
        assert_eq!(client.backoff_for(2), Duration::from_secs(2));
        assert_eq!(client.backoff_for(3), Duration::from_secs(4));
        assert_eq!(client.backoff_for(5), Duration::from_secs(16));
        assert_eq!(client.backoff_for(6), Duration::from_secs(30));
        assert_eq!(client.backoff_for(99), Duration::from_secs(30));
    }

    #[test]
    fn read_line_splits_crlf_and_flushes_tail() {
        // 用一个同步的 Stream 测试 read_line 的缓冲逻辑
        use futures_util::stream;
        let stream = stream::iter(vec![
            Ok::<_, reqwest::Error>(bytes::Bytes::from_static(b"data: a\r\n")),
            Ok(bytes::Bytes::from_static(b"data: b")),
        ]);
        let mut stream = std::pin::pin!(stream);
        let mut buffer = Vec::new();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let first = read_line(&mut stream, &mut buffer).await.unwrap().unwrap();
            assert_eq!(first, b"data: a".to_vec());
            // 尾部无换行，应在 EOF 时冲刷
            let tail = read_line(&mut stream, &mut buffer).await.unwrap().unwrap();
            assert_eq!(tail, b"data: b".to_vec());
            let eof = read_line(&mut stream, &mut buffer).await.unwrap();
            assert!(eof.is_none());
        });
    }
}
