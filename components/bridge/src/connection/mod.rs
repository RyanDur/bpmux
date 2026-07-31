use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

use super::{WsSink, event_subscription};
use protocol::ServerMessage;

mod frame;

pub async fn run_ws_connection(
    port: u16,
    name: Arc<Mutex<String>>,
    user_id: Option<String>,
    ws_tx_store: Arc<Mutex<Option<WsSink>>>,
    subscriptions: event_subscription::SubscriptionRegistry,
) {
    let ws_url = local_url::ws_url(port, "/ws");
    let (ws_stream, _) = match tokio_tungstenite::connect_async(&ws_url).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Bridge: failed to connect to bpmux WebSocket: {e}");
            return;
        }
    };

    let (ws_tx, mut ws_rx) = ws_stream.split();
    *ws_tx_store.lock().await = Some(ws_tx);

    if !send_identify(&ws_tx_store, &name, &user_id, &subscriptions).await {
        return;
    }

    match ws_rx.next().await {
        Some(Ok(_welcome)) => {
            eprintln!("Bridge: connected to bpmux on port {port}");
        }
        Some(Err(e)) => {
            eprintln!("Bridge: welcome frame error: {e}");
            return;
        }
        None => {
            eprintln!("Bridge: connection closed before welcome");
            return;
        }
    }

    while let Some(frame_result) = ws_rx.next().await {
        let frame = match frame_result {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Bridge: websocket receive error: {e}");
                break;
            }
        };
        let text = match frame {
            tungstenite::Message::Text(t) => t,
            tungstenite::Message::Close(_) => {
                eprintln!("Bridge: server closed connection");
                break;
            }
            tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) => continue,
            other => {
                eprintln!(
                    "Bridge: unexpected frame type ({} bytes), dropping",
                    match &other {
                        tungstenite::Message::Binary(b) => b.len(),
                        _ => 0,
                    }
                );
                continue;
            }
        };
        let msg: ServerMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Bridge: failed to parse server message: {e} (text: {text})");
                continue;
            }
        };
        if let ServerMessage::Error { ref reason } = msg {
            eprintln!("Bridge: server rejected message: {reason}");
            continue;
        }
        frame::handle_message_frame(msg, &name, user_id.as_deref(), &subscriptions).await;
    }

    *ws_tx_store.lock().await = None;
    eprintln!("Bridge: disconnected from bpmux");
}

async fn send_identify(
    ws_tx_store: &Arc<Mutex<Option<WsSink>>>,
    name: &Arc<Mutex<String>>,
    user_id: &Option<String>,
    subscriptions: &event_subscription::SubscriptionRegistry,
) -> bool {
    let current_name = name.lock().await.clone();
    let cursor = subscriptions.cursor().await;
    let mut tx = ws_tx_store.lock().await;
    let Some(ref mut tx) = *tx else {
        return true;
    };
    let identify = json!({"type": "identify", "name": current_name, "tool": "claude-code", "cursor": cursor, "user_id": user_id.clone()})
        .to_string();
    if let Err(error) = tx.send(tungstenite::Message::Text(identify.into())).await {
        eprintln!("Bridge: failed to identify: {error}");
        return false;
    }
    true
}
