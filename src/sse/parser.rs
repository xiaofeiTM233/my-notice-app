//! 增量 SSE（Server-Sent Events）报文解析器。
//!
//! 与 [`reqwest`](https://docs.rs/reqwest) 的字节流配合使用：`feed()` 逐块喂入
//! 原始字节，跨块边界自动缓冲，累积完成后产生 [`SseMessage`]。

use std::collections::VecDeque;

/// 一条完整的 SSE 事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseMessage {
    /// `event:` 字段，未声明时为 `None`。
    pub event: Option<String>,
    /// `data:` 字段拼接结果（多行 data 以 `\n` 连接）。
    pub data: String,
    /// `id:` 字段，未声明时为 `None`。
    pub id: Option<String>,
    /// `retry:` 重连间隔（毫秒）。
    pub retry: Option<u64>,
}

/// 增量解析器：状态保存在内部，可跨多次 `feed` 调用。
#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
    pending: VecDeque<SseMessage>,
    event: Option<String>,
    data: String,
    id: Option<String>,
    retry: Option<u64>,
    saw_data: bool,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// 喂入一段字节。完成一条或多条事件后，事件被排入内部队列，
    /// 可通过 [`Self::drain`] 取出。
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);

        // 找到最后一个换行符位置：其之前都是完整行。
        let mut last_newline = None;
        for (idx, byte) in self.buffer.iter().enumerate() {
            if *byte == b'\n' {
                last_newline = Some(idx);
            }
        }

        if let Some(nl) = last_newline {
            // split_off 之后 buffer 中只保留到最后一个 \n（含），
            // 剩余部分是尚未结束的半行，留到下一次 feed。
            let rest = self.buffer.split_off(nl + 1);
            let complete: Vec<Vec<u8>> = self
                .buffer
                .split(|b| *b == b'\n')
                .map(|line| line.to_vec())
                .collect();
            for line in complete {
                self.process_line(&line);
            }
            self.buffer = rest;
        }
    }

    /// 取出所有已完成的 SSE 事件。
    pub fn drain(&mut self) -> Vec<SseMessage> {
        self.pending.drain(..).collect()
    }

    /// 处理单行内容（不含行尾换行符）。
    fn process_line(&mut self, raw_line: &[u8]) {
        let mut line = raw_line;
        // 兼容 `\r\n`
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }

        if line.is_empty() {
            self.finalize_event();
            return;
        }

        // 以 `:` 开头的行是注释/心跳（keep-alive），忽略。
        if line.first() == Some(&b':') {
            return;
        }

        let text = String::from_utf8_lossy(line);
        let (field, value) = match text.find(':') {
            Some(idx) => {
                let field = &text[..idx];
                let mut value = &text[idx + 1..];
                // 规范：冒号后的单个空格应被忽略。
                if let Some(stripped) = value.strip_prefix(' ') {
                    value = stripped;
                }
                (field, value)
            }
            None => (text.as_ref(), ""),
        };

        match field {
            "event" => self.event = Some(value.to_owned()),
            "data" => {
                self.saw_data = true;
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(value);
            }
            "id" => self.id = Some(value.to_owned()),
            "retry" => {
                if let Ok(retry) = value.trim().parse::<u64>() {
                    self.retry = Some(retry);
                }
            }
            _ => {}
        }
    }

    /// 空行触发事件结束：若积累了 data 或显式声明了 event，则产出事件。
    fn finalize_event(&mut self) {
        let has_data = self.saw_data || !self.data.is_empty();
        if has_data || self.event.is_some() {
            self.pending.push_back(SseMessage {
                event: self.event.take(),
                data: std::mem::take(&mut self.data),
                id: self.id.take(),
                retry: self.retry.take(),
            });
        }
        self.event = None;
        self.id = None;
        self.retry = None;
        self.saw_data = false;
        self.data.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_once(parser: &mut SseParser, bytes: &str) -> Vec<SseMessage> {
        parser.feed(bytes.as_bytes());
        parser.drain()
    }

    #[test]
    fn parse_single_event() {
        let mut parser = SseParser::new();
        let msgs = feed_once(&mut parser, "event: notification\ndata: {\"id\":\"1\"}\n\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].event.as_deref(), Some("notification"));
        assert_eq!(msgs[0].data, r#"{"id":"1"}"#);
    }

    #[test]
    fn parse_multiline_data() {
        let mut parser = SseParser::new();
        let msgs = feed_once(&mut parser, "data: line1\ndata: line2\n\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].data, "line1\nline2");
    }

    #[test]
    fn parse_multiple_events_in_one_chunk() {
        let mut parser = SseParser::new();
        let msgs = feed_once(&mut parser, "data: a\n\ndata: b\n\n");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].data, "a");
        assert_eq!(msgs[1].data, "b");
    }

    #[test]
    fn chunk_boundary_split() {
        let mut parser = SseParser::new();
        // 一行数据被 TCP 分包切成两半：前半到达时不应产生事件。
        assert!(feed_once(&mut parser, "data: hel").is_empty());
        let msgs = feed_once(&mut parser, "lo\n\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].data, "hello");
    }

    #[test]
    fn comment_lines_are_ignored() {
        let mut parser = SseParser::new();
        let msgs = feed_once(&mut parser, ": keepalive\n\n: another\n");
        assert!(msgs.is_empty());
    }

    #[test]
    fn crlf_line_endings() {
        let mut parser = SseParser::new();
        let msgs = feed_once(&mut parser, "data: hello\r\ndata: world\r\n\r\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].data, "hello\nworld");
    }

    #[test]
    fn id_and_retry_fields() {
        let mut parser = SseParser::new();
        let msgs = feed_once(&mut parser, "id: 42\nretry: 3000\ndata: x\n\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id.as_deref(), Some("42"));
        assert_eq!(msgs[0].retry, Some(3000));
    }

    #[test]
    fn field_without_colon_is_empty_value() {
        let mut parser = SseParser::new();
        let msgs = feed_once(&mut parser, "data\n\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].data, "");
    }

    #[test]
    fn utf8_multibyte_across_chunks() {
        let mut parser = SseParser::new();
        assert!(feed_once(&mut parser, "data: 你好").is_empty());
        let msgs = feed_once(&mut parser, "世界\n\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].data, "你好世界");
    }

    #[test]
    fn event_without_data_is_dispatched_when_named() {
        let mut parser = SseParser::new();
        let msgs = feed_once(&mut parser, "event: ping\n\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].event.as_deref(), Some("ping"));
        assert_eq!(msgs[0].data, "");
    }
}
