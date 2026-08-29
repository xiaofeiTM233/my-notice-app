use crate::config::BackendCfg;
use crate::model::{self, Msg};
use std::sync::mpsc::Sender;
use std::time::Duration;
use tungstenite::Message;

pub enum Ev {
    N(Msg),
    St(&'static str),
    PopDone,
}

pub fn spawn(cfg: BackendCfg, tx: Sender<Ev>) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("backend".into())
        .spawn(move || match cfg.mode.as_str() {
            "poll" => poll_loop(cfg, tx),
            _ => ws_loop(cfg, tx),
        })
        .expect("spawn backend thread")
}

fn ws_loop(cfg: BackendCfg, tx: Sender<Ev>) {
    loop {
        let _ = tx.send(Ev::St("connecting"));
        match tungstenite::connect(&cfg.url) {
            Ok((mut ws, _)) => {
                eprintln!("backend: ws 已连接 {}", cfg.url);
                let _ = tx.send(Ev::St("connected"));
                match ws.get_ref() {
                    tungstenite::stream::MaybeTlsStream::Plain(s) => {
                        let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
                    }
                    tungstenite::stream::MaybeTlsStream::Rustls(s) => {
                        let _ = s.get_ref().set_read_timeout(Some(Duration::from_secs(5)));
                    }
                    _ => {}
                }
                loop {
                    match ws.read() {
                        Ok(Message::Text(t)) => push(&tx, t.as_str()),
                        Ok(Message::Ping(_) | Message::Pong(_)) => {}
                        Ok(Message::Close(_)) => break,
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(ref e))
                            if e.kind() == std::io::ErrorKind::WouldBlock =>
                        {
                            let _ = ws.flush(); // 触发自动 pong
                        }
                        Err(tungstenite::Error::Io(e))
                            if e.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(e) => {
                            eprintln!("backend: ws 断开({e})");
                            break;
                        }
                    }
                }
            }
            Err(e) => eprintln!("backend: ws 连接失败({e})"),
        }
        let _ = tx.send(Ev::St("offline"));
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn poll_loop(cfg: BackendCfg, tx: Sender<Ev>) {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .new_agent();
    let mut since = model::now();
    loop {
        let _ = tx.send(Ev::St("polling"));
        let mut req = agent.get(&cfg.url).query("since", since.to_string());
        if !cfg.token.is_empty() {
            req = req.header("Authorization", &format!("Bearer {}", cfg.token));
        }
        match req.call() {
            Ok(mut res) => match res.body_mut().read_to_string() {
                Ok(s) => {
                    // 兼容裸数组与 {"items": [...]}
                    let msgs: Vec<Msg> = serde_json::from_str::<Vec<Msg>>(&s).unwrap_or_else(|_| {
                        serde_json::from_str::<Items>(&s).map(|r| r.items).unwrap_or_default()
                    });
                    for m in msgs {
                        if m.ts > since {
                            since = m.ts;
                        }
                        let _ = tx.send(Ev::N(m));
                    }
                }
                Err(e) => eprintln!("backend: 响应读取失败({e})"),
            },
            Err(e) => eprintln!("backend: 轮询失败({e})"),
        }
        std::thread::sleep(Duration::from_secs(cfg.poll_secs.max(1)));
    }
}

#[derive(serde::Deserialize)]
struct Items {
    #[serde(default)]
    items: Vec<Msg>,
}

fn push(tx: &Sender<Ev>, s: &str) {
    match serde_json::from_str::<Msg>(s) {
        Ok(m) => {
            let _ = tx.send(Ev::N(m));
        }
        Err(e) => eprintln!("backend: 消息解析失败({e}): {s}"),
    }
}
