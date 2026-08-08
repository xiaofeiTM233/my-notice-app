use crate::config::{Feed, Mode};
use crate::model::{Notif, Payload};
use anyhow::Result;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tungstenite::client::IntoClientRequest;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

pub enum Ev {
    Got(Notif),
    /// 连接状态变化：是否在线 + 描述
    Up(bool, String),
}

pub struct Handle {
    stop: Arc<AtomicBool>,
}

impl Handle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// 后台单线程拉取通知，断线按指数退避重连。sink 在该线程内被调用，
/// 调用方负责切回 UI 线程。
pub fn spawn<F>(cfg: Feed, sink: F) -> Handle
where
    F: Fn(Ev) + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    if cfg.mode == Mode::Off {
        return Handle { stop };
    }
    let s = stop.clone();
    let _ = std::thread::Builder::new()
        .name("feed".into())
        .spawn(move || {
            let mut wait = cfg.retry_ms.max(200);
            while !s.load(Ordering::Relaxed) {
                let r = match cfg.mode {
                    Mode::Ws => ws(&cfg, &s, &sink),
                    Mode::Poll => poll(&cfg, &s, &sink),
                    Mode::Off => return,
                };
                if s.load(Ordering::Relaxed) {
                    break;
                }
                let (connected, why) = match r {
                    Ok(c) => (c, "连接已断开".to_string()),
                    Err(e) => (false, e.to_string()),
                };
                sink(Ev::Up(false, why));
                if connected {
                    wait = cfg.retry_ms.max(200);
                }
                nap(&s, wait);
                wait = (wait * 2).min(cfg.retry_max_ms.max(wait));
            }
        });
    Handle { stop }
}

fn ws<F: Fn(Ev)>(cfg: &Feed, stop: &AtomicBool, sink: &F) -> Result<bool> {
    let mut req = cfg.url.as_str().into_client_request()?;
    if !cfg.token.is_empty() {
        let v = tungstenite::http::HeaderValue::from_str(&format!("Bearer {}", cfg.token))?;
        req.headers_mut().insert("Authorization", v);
    }
    let (mut sock, _) = tungstenite::connect(req)?;
    // 短读超时让线程能及时响应退出请求
    timeout(&sock, Duration::from_millis(400));
    sink(Ev::Up(true, format!("已连接 {}", cfg.url)));

    loop {
        if stop.load(Ordering::Relaxed) {
            let _ = sock.close(None);
            return Ok(true);
        }
        match sock.read() {
            Ok(Message::Text(t)) => feed(t.as_str(), sink),
            Ok(Message::Binary(b)) => {
                if let Ok(t) = std::str::from_utf8(&b) {
                    feed(t, sink);
                }
            }
            Ok(Message::Close(_)) => return Ok(true),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

fn poll<F: Fn(Ev)>(cfg: &Feed, stop: &AtomicBool, sink: &F) -> Result<bool> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_millis(cfg.poll_timeout_ms.max(1000))))
        .build()
        .into();
    let mut since = crate::model::now();
    let mut live = false;

    while !stop.load(Ordering::Relaxed) {
        let mut req = agent.get(with_since(&cfg.poll_url, since));
        if !cfg.token.is_empty() {
            req = req.header("Authorization", &format!("Bearer {}", cfg.token));
        }
        let mut resp = req.call()?;
        let txt = resp.body_mut().read_to_string()?;
        if !live {
            live = true;
            sink(Ev::Up(true, format!("已连接 {}", cfg.poll_url)));
        }
        for n in parse(&txt) {
            since = since.max(n.ts);
            sink(Ev::Got(n));
        }
        nap(stop, cfg.poll_ms);
    }
    Ok(live)
}

fn feed<F: Fn(Ev)>(txt: &str, sink: &F) {
    for n in parse(txt) {
        sink(Ev::Got(n));
    }
}

fn parse(txt: &str) -> Vec<Notif> {
    let t = txt.trim();
    if t.is_empty() {
        return vec![];
    }
    match serde_json::from_str::<Payload>(t) {
        Ok(p) => p
            .into_vec()
            .into_iter()
            .filter(|n| !(n.title.is_empty() && n.body.is_empty()))
            .collect(),
        Err(_) => vec![],
    }
}

fn with_since(base: &str, since: i64) -> String {
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{base}{sep}since={since}")
}

fn timeout(sock: &WebSocket<MaybeTlsStream<TcpStream>>, d: Duration) {
    // 仅对明文 TCP 设置读超时，使读循环能及时响应退出信号；
    // TLS 流不在此处理（不影响功能，仅停止响应略慢）。MaybeTlsStream 非穷尽，
    // 用 if let 避免对具体变体字段做不可靠的访问。
    if let MaybeTlsStream::Plain(s) = sock.get_ref() {
        let _ = s.set_read_timeout(Some(d));
    }
}

fn nap(stop: &AtomicBool, ms: u64) {
    let mut left = ms;
    while left > 0 && !stop.load(Ordering::Relaxed) {
        let step = left.min(100);
        std::thread::sleep(Duration::from_millis(step));
        left -= step;
    }
}
