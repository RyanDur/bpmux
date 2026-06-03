use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use rmcp::RoleServer;
use rmcp::model::{CustomNotification, ServerNotification};
use rmcp::service::Peer;
use serde_json::json;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

use super::{
    WsSink, build_notification_meta, event_subscription, message_type_reaches_llm_stream,
    validate_meta, ws_send_client,
};
use crate::session::message::DeliveryStatus;
use crate::transport::protocol::{ClientMessage, ServerMessage};

pub(crate) async fn run_ws_connection(
    port: u16,
    name: Arc<Mutex<String>>,
    peer: Peer<RoleServer>,
    ws_tx_store: Arc<Mutex<Option<WsSink>>>,
    subscriptions: event_subscription::SubscriptionRegistry,
    delivered_pending: Arc<Mutex<Vec<String>>>,
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
        let mut tx = ws_tx_store.lock().await;
        if let Some(ref mut tx) = *tx {
            let identify = json!({"type": "identify", "name": current_name, "tool": "claude-code"})
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
            id,
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
            match state_delta.as_deref() {
                Some(crate::session::message::StateDeltaPayload::BoardInvalidated { reason }) => {
                    subscriptions
                        .dispatch("board_invalidated", json!({ "reason": reason }))
                        .await;
                    continue;
                }
                Some(crate::session::message::StateDeltaPayload::Ci { snapshot }) => {
                    subscriptions
                        .dispatch(
                            "ci_state_changed",
                            json!({
                                "conclusion": serde_json::to_value(snapshot.state)
                                    .unwrap_or(serde_json::Value::Null),
                                "run_id": snapshot.run_id,
                                "head_sha": snapshot.head_sha,
                                "display_title": snapshot.display_title,
                                "url": crate::ci::github_actions_url(),
                            }),
                        )
                        .await;
                    continue;
                }
                Some(crate::session::message::StateDeltaPayload::Sync { transition }) => {
                    subscriptions
                        .dispatch(
                            "sync_status_changed",
                            json!({
                                "status": transition.status,
                                "why": transition.why,
                                "pending_events": transition.pending_events,
                                "conflicts_pending": transition.conflicts_pending,
                            }),
                        )
                        .await;
                    continue;
                }
                Some(crate::session::message::StateDeltaPayload::StoryTransition {
                    transition,
                }) => {
                    subscriptions
                        .dispatch(
                            "story_state_transition",
                            json!({
                                "story_id": transition.story_id,
                                "from_state": transition.from_state,
                                "target_state": transition.target_state,
                                "actor": transition.actor,
                                "ts": ts,
                            }),
                        )
                        .await;
                    continue;
                }
                Some(crate::session::message::StateDeltaPayload::MessageStatus { .. }) => {
                    continue;
                }
                _ => {}
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
            let relevant = match &to {
                None => true,
                Some(recipients)
                    if recipients
                        .iter()
                        .any(|r| r == crate::session::message::BROADCAST_ALL) =>
                {
                    true
                }
                Some(recipients) => recipients.iter().any(|r| r == current_name.as_str()),
            };
            let specifically_addressed = matches!(
                &to,
                Some(recipients)
                    if !recipients
                        .iter()
                        .any(|r| r == crate::session::message::BROADCAST_ALL)
                        && recipients.iter().any(|r| r == current_name.as_str())
            );
            let channel = match &to {
                None => "broadcast".to_string(),
                Some(recipients)
                    if recipients
                        .iter()
                        .any(|r| r == crate::session::message::BROADCAST_ALL) =>
                {
                    "broadcast".to_string()
                }
                Some(recipients) => recipients.join(","),
            };
            drop(current_name);
            if !relevant {
                continue;
            }

            let meta = build_notification_meta(
                &from,
                &ts,
                &channel,
                addressee.as_deref(),
                message_type.as_deref(),
            );
            if let Err(violation) = validate_meta(&meta) {
                eprintln!("Bridge: CONTRACT VIOLATION on outbound notification: {violation}");
                continue;
            }
            let notification = CustomNotification::new(
                "notifications/claude/channel",
                Some(json!({
                    "content": format!("{from_short}: {text}"),
                    "meta": meta
                })),
            );

            if let Err(e) = peer
                .send_notification(ServerNotification::CustomNotification(notification))
                .await
            {
                eprintln!("Bridge: failed to send channel notification: {e}");
                break;
            }
            if specifically_addressed && !id.is_empty() {
                delivered_pending.lock().await.push(id.clone());
                ws_send_client(
                    &ws_tx_store,
                    &ClientMessage::MarkReceipt {
                        message_id: id,
                        status: DeliveryStatus::Delivered,
                    },
                )
                .await;
            }

            if let Some(directed) = addressee
                .as_deref()
                .filter(|a| *a != crate::session::message::BROADCAST_ALL)
            {
                subscriptions
                    .dispatch(
                        "addressed_to_me",
                        json!({
                            "from": from_short,
                            "text": text,
                            "ts": ts,
                            "channel": channel,
                            "addressee": directed,
                        }),
                    )
                    .await;
            }
        }
    }

    *ws_tx_store.lock().await = None;
    eprintln!("Bridge: disconnected from bpmux");
}
