//! SSE 客户端与解析器。

pub mod client;
pub mod parser;

pub use client::{ConnectionState, SseClient, SseClientEvent, SseConfig};
pub use parser::{SseMessage, SseParser};
