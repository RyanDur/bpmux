use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

use super::{WsSink, event_subscription, message_type_reaches_llm_stream};
use crate::transport::protocol::ServerMessage;

pub(crate) async fn run_ws_connection(
    port: u16,
    name: Arc<Mutex<String>>,
    ws_tx_store: Arc<Mutex<Option<WsSink>>>,
    subscriptions: event_subscription::SubscriptionRegistry,
) {
    let ws_url = crate::transport::local_url::ws_url(port, "/ws");
    let (ws_stream, _) = match tokio_tungstenite::connect_async(&ws_url).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Bridge: failed to connect to bpmux WebSocket: {e}");
            return;
        }
    };

    let (ws_tx, mut ws_rx) = ws_stream.split();
    *ws_tx_store.lock().await = Some(ws_tx);

    {
        let current_name = name.lock().await.clone();
        let cursor = subscriptions.cursor().await;
        let mut tx = ws_tx_store.lock().await;
        if let Some(ref mut tx) = *tx {
            let identify = json!({"type": "identify", "name": current_name, "tool": "claude-code", "cursor": cursor})
                .to_string();
            if let Err(e) = tx.send(tungstenite::Message::Text(identify.into())).await {
                eprintln!("Bridge: failed to identify: {e}");
                return;
            }
        }
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
        if let ServerMessage::Message {
            from,
            text,
            ts,
            to,
            addressee,
            message_type,
            multi_depth: _,
            state_delta,
            state_snapshot: _,
        } = msg
        {
            if state_delta.is_some() {
                subscriptions.advance_cursor(&ts).await;
            }
            if let Some((event, payload)) = state_delta
                .as_deref()
                .and_then(|delta| delta.to_subscription_event(&ts))
            {
                subscriptions.dispatch(event, payload).await;
                continue;
            }
            let from_short = from.split(':').next().unwrap_or(&from);
            let current_name = name.lock().await;
            if from_short == *current_name {
                continue;
            }
            drop(current_name);

            if !message_type_reaches_llm_stream(message_type.as_deref()) {
                continue;
            }

            let current_name = name.lock().await;
            let relevant = crate::bridge::chat_relevance::scope_reaches_live_stream(
                to.as_deref(),
                &current_name,
            );
            let channel = match &to {
                None => "broadcast".to_string(),
                Some(recipients)
                    if recipients
                        .iter()
                        .any(|r| r == crate::message::BROADCAST_ALL) =>
                {
                    "broadcast".to_string()
                }
                Some(recipients) => recipients.join(","),
            };
            drop(current_name);
            if !relevant {
                continue;
            }

            let current_name = name.lock().await;
            let addressed_to_me = addressee.as_deref() == Some(current_name.as_str())
                || to.as_ref().is_some_and(|recipients| {
                    recipients
                        .iter()
                        .any(|r| r != crate::message::BROADCAST_ALL && r == current_name.as_str())
                });
            drop(current_name);
            let sender_is_owner = crate::transport::ws::role_from_short_name(from_short)
                == crate::store::RosterRole::Owner;
            let declared = subscriptions.declared_chat_classes().await;
            if !crate::bridge::chat_relevance::chat_class_reaches_live_stream(
                message_type.as_deref().unwrap_or("chat"),
                addressed_to_me,
                sender_is_owner,
                &declared,
            ) {
                continue;
            }

            for class in chat_event_classes(&channel, addressee.as_deref()) {
                let mut payload = json!({
                    "from": from_short,
                    "text": &text,
                    "ts": &ts,
                    "channel": &channel,
                });
                if class == "addressed_to_me"
                    && let Some(a) = addressee.as_deref()
                    && let Some(obj) = payload.as_object_mut()
                {
                    obj.insert("addressee".to_string(), json!(a));
                }
                subscriptions.dispatch(class, payload).await;
            }
        }
    }

    *ws_tx_store.lock().await = None;
    eprintln!("Bridge: disconnected from bpmux");
}

pub(crate) fn chat_event_classes(channel: &str, addressee: Option<&str>) -> Vec<&'static str> {
    let mut classes = Vec::new();
    if channel == "broadcast" {
        classes.push("broadcast_received");
    }
    if matches!(addressee, Some(a) if a != crate::message::BROADCAST_ALL) {
        classes.push("addressed_to_me");
    }
    classes
}
