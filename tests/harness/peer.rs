//! Direct WebSocket peer for tests that need to act as a raw bpmux client
//! (no bridge, no MCP). Use `WsPeer::connect` to identify and stay alive;
//! use `send`/`next_text` to exchange messages.

use std::time::Duration;

use bpmux::transport::protocol::ServerMessage;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message as WsMessage,
};

use super::pty::TestBpmux;

type WsSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = SplitSink<WsSocket, WsMessage>;
type WsStream = SplitStream<WsSocket>;

/// A raw WebSocket client connected to a `TestBpmux`, identified under `name`.
pub struct WsPeer {
    tx: WsSink,
    rx: WsStream,
}

impl WsPeer {
    /// Connect, identify, and consume the welcome frame.
    pub async fn connect(bpmux: &TestBpmux, name: &str) -> Self {
        let ws_url = format!("ws://127.0.0.1:{}/ws", bpmux.port());
        let ws_stream = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match connect_async(&ws_url).await {
                    Ok((stream, _)) => return stream,
                    Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
                }
            }
        })
        .await
        .expect("WebSocket should connect");
        let (mut tx, mut rx) = ws_stream.split();
        let identify = serde_json::json!({"type": "identify", "name": name}).to_string();
        tx.send(WsMessage::Text(identify.into()))
            .await
            .expect("send identify");
        // consume welcome
        let _ = rx.next().await.expect("receive welcome").expect("welcome");
        Self { tx, rx }
    }

    /// Send a `send` command to bpmux.
    pub async fn send(&mut self, text: &str) {
        let msg = serde_json::json!({"type": "send", "text": text}).to_string();
        self.tx
            .send(WsMessage::Text(msg.into()))
            .await
            .expect("send");
    }

    /// Next text frame, stripped of non-text frames. Returns None on close.
    pub async fn next_text(&mut self) -> Option<String> {
        while let Some(Ok(frame)) = self.rx.next().await {
            if let WsMessage::Text(t) = frame {
                return Some(t.to_string());
            }
        }
        None
    }

    /// Wait for the next user-sent `Message` frame (skipping welcome, pong,
    /// errors, and bpmux's own system announcements). Returns `(from, text)`
    /// or panics on timeout.
    pub async fn recv_message(&mut self, timeout: Duration) -> (String, String) {
        let result = tokio::time::timeout(timeout, async {
            loop {
                let Some(raw) = self.next_text().await else {
                    panic!("websocket closed before a user message arrived");
                };
                if let Some(msg) = extract_user_message(&raw) {
                    return msg;
                }
            }
        })
        .await;
        result.expect("timed out waiting for user message")
    }
}

/// Parse a WebSocket text frame and return `(from, text)` if it represents
/// a user `Message` (not a system announcement from "bpmux", not welcome/
/// pong/error). Returns `None` for anything that should be skipped.
fn extract_user_message(raw: &str) -> Option<(String, String)> {
    match serde_json::from_str::<ServerMessage>(raw).ok()? {
        ServerMessage::Message { from, text, .. } if from != "bpmux" => Some((from, text)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_is_extracted() {
        let raw = r#"{"type":"message","from":"dev","text":"hello","ts":"2026-04-05T00:00:00Z"}"#;
        assert_eq!(
            extract_user_message(raw),
            Some(("dev".to_string(), "hello".to_string()))
        );
    }

    #[test]
    fn system_message_from_bpmux_is_skipped() {
        let raw =
            r#"{"type":"message","from":"bpmux","text":"bot joined","ts":"2026-04-05T00:00:00Z"}"#;
        assert_eq!(extract_user_message(raw), None);
    }

    #[test]
    fn welcome_frame_is_skipped() {
        let raw = r#"{"type":"welcome","session_id":"abc-123"}"#;
        assert_eq!(extract_user_message(raw), None);
    }

    #[test]
    fn pong_frame_is_skipped() {
        let raw = r#"{"type":"pong"}"#;
        assert_eq!(extract_user_message(raw), None);
    }

    #[test]
    fn error_frame_is_skipped() {
        let raw = r#"{"type":"error","reason":"nope"}"#;
        assert_eq!(extract_user_message(raw), None);
    }

    #[test]
    fn malformed_frame_is_skipped() {
        assert_eq!(extract_user_message("not json"), None);
    }
}
