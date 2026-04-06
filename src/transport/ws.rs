use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::session;
use crate::transport::protocol::{ClientMessage, ServerMessage};

const MAX_MESSAGE_SIZE: usize = 4096;
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(session): State<Arc<session::Session>>,
) -> Response {
    ws.on_upgrade(|socket| handle_ws(socket, session))
}

async fn handle_ws(socket: WebSocket, session: Arc<session::Session>) {
    let (tx, rx) = socket.split();
    if let Err(e) = run_ws_session(tx, rx, session).await {
        eprintln!("WebSocket session error: {e}");
    }
}

async fn run_ws_session(
    mut tx: SplitSink<WebSocket, WsMessage>,
    mut rx: SplitStream<WebSocket>,
    session: Arc<session::Session>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut identity = wait_for_identify(&mut tx, &mut rx).await?;
    session.peer_connected();
    let mut broadcast_rx = session.subscribe();
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.tick().await;
    send_json(
        &mut tx,
        &ServerMessage::Welcome {
            session_id: session.id.clone(),
        },
    )
    .await?;
    // Deliver pinned items before message history (theory-first reconnect)
    if let Ok(pins) = session.read_pins()
        && !pins.is_empty()
    {
        let items: Vec<String> = pins
            .iter()
            .enumerate()
            .map(|(i, p)| format!("  {}. {}", i + 1, p))
            .collect();
        let pin_msg = ServerMessage::Message {
            from: "bpmux".to_string(),
            text: format!("Pinned:\n{}", items.join("\n")),
            ts: chrono::Utc::now().to_rfc3339(),
            to: None,
            message_type: Some("system".to_string()),
        };
        send_json(&mut tx, &pin_msg).await?;
    }
    // Deliver handoff summary from previous session if available
    let peer_short = short_name(&identity).to_string();
    if let Ok(Some(handoff)) = session.read_handoff(&peer_short) {
        let handoff_msg = ServerMessage::Message {
            from: "bpmux".to_string(),
            text: format!("Handoff from your last session:\n{handoff}"),
            ts: chrono::Utc::now().to_rfc3339(),
            to: None,
            message_type: Some("system".to_string()),
        };
        send_json(&mut tx, &handoff_msg).await?;
    }
    let join_msg = session::Message {
        from: "bpmux".to_string(),
        text: format!("{identity} joined"),
        ts: chrono::Utc::now(),
        message_type: Some("system".to_string()),
        to: None,
    };
    session.broadcast(&join_msg)?;
    let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
        loop {
            tokio::select! {
                broadcast_result = broadcast_rx.recv() => {
                    match broadcast_result {
                        Ok(msg) => {
                            if should_deliver(&msg, &identity) {
                                send_json(&mut tx, &ServerMessage::from(msg)).await?;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!("WebSocket client {identity} lagged by {n} messages");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = heartbeat.tick() => {
                    if tx.send(WsMessage::Ping(vec![].into())).await.is_err() {
                        break;
                    }
                }
                ws_result = rx.next() => {
                    match ws_result {
                        Some(Ok(WsMessage::Text(text))) => {
                            if let Some(new_identity) = handle_client_message(&text, &identity, &session, &mut tx).await? {
                                identity = new_identity;
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) | None => break,
                        Some(Err(e)) => { eprintln!("WebSocket read error for {identity}: {e}"); break; }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
    .await;
    let leave_msg = session::Message {
        from: "bpmux".to_string(),
        text: format!("{identity} left"),
        ts: chrono::Utc::now(),
        message_type: Some("system".to_string()),
        to: None,
    };
    let _ = session.broadcast(&leave_msg);
    session.peer_disconnected();
    result
}

async fn wait_for_identify(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    rx: &mut SplitStream<WebSocket>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = rx.next().await {
            if let WsMessage::Text(text) = msg {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Identify { name, tool }) => {
                        let tool_name = tool.unwrap_or_else(|| "ws".to_string());
                        return Ok(format!("{name}:{tool_name}"));
                    }
                    Ok(_) => {
                        send_json(
                            tx,
                            &ServerMessage::Error {
                                reason: "must identify first".to_string(),
                            },
                        )
                        .await?;
                    }
                    Err(e) => {
                        send_json(
                            tx,
                            &ServerMessage::Error {
                                reason: format!("invalid message: {e}"),
                            },
                        )
                        .await?;
                    }
                }
            }
        }
        Err("connection closed before identify".into())
    })
    .await;
    match timeout {
        Ok(result) => result,
        Err(_) => Err("identify timeout (5s)".into()),
    }
}

/// Short name for identity matching: "dev:claude-code" → "dev".
fn short_name(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

/// Decide whether a message should be pushed to a given peer.
/// Broadcast (to=None) → everyone. Addressed (to=Some) → only
/// named recipients and the original sender.
fn should_deliver(msg: &session::Message, peer_identity: &str) -> bool {
    let recipients = match &msg.to {
        None => return true,
        Some(r) => r,
    };
    // to:["all"] is explicit broadcast
    if recipients.iter().any(|r| r == "all") {
        return true;
    }
    let peer_short = short_name(peer_identity);
    let sender_short = short_name(&msg.from);
    if peer_short == sender_short {
        return true;
    }
    recipients.iter().any(|r| r == peer_short)
}

/// Returns Some(new_identity) if the client renamed itself.
async fn handle_client_message(
    text: &str,
    identity: &str,
    session: &session::Session,
    tx: &mut SplitSink<WebSocket, WsMessage>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    if text.len() > MAX_MESSAGE_SIZE {
        send_json(
            tx,
            &ServerMessage::Error {
                reason: "message too large".to_string(),
            },
        )
        .await?;
        return Ok(None);
    }
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::Send {
            text: msg_text,
            to,
            message_type,
        }) => {
            let msg = session::Message {
                from: identity.to_string(),
                text: msg_text,
                ts: chrono::Utc::now(),
                message_type,
                to,
            };
            session.broadcast(&msg)?;
        }
        Ok(ClientMessage::Ping) => {
            send_json(tx, &ServerMessage::Pong).await?;
        }
        Ok(ClientMessage::Rename { name }) => {
            let tool = identity.split(':').nth(1).unwrap_or("ws");
            return Ok(Some(format!("{name}:{tool}")));
        }
        Ok(ClientMessage::Handoff { text: handoff_text }) => {
            let peer_short = short_name(identity);
            let _ = session.save_handoff(peer_short, &handoff_text);
        }
        Ok(ClientMessage::Identify { .. }) => {
            send_json(
                tx,
                &ServerMessage::Error {
                    reason: "already identified".to_string(),
                },
            )
            .await?;
        }
        Err(e) => {
            send_json(
                tx,
                &ServerMessage::Error {
                    reason: format!("invalid message: {e}"),
                },
            )
            .await?;
        }
    }
    Ok(None)
}

async fn send_json(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    msg: &ServerMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string(msg)?;
    tx.send(WsMessage::Text(json.into())).await?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn msg(from: &str, to: Option<Vec<&str>>) -> session::Message {
        session::Message {
            from: from.to_string(),
            text: "test".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: to.map(|v| v.into_iter().map(String::from).collect()),
        }
    }

    #[test]
    fn broadcast_delivered_to_everyone() {
        assert!(should_deliver(
            &msg("dev:claude-code", None),
            "pm:claude-code"
        ));
        assert!(should_deliver(&msg("dev:claude-code", None), "ryan:human"));
    }

    #[test]
    fn addressed_delivered_to_recipient() {
        let m = msg("dev:claude-code", Some(vec!["pm"]));
        assert!(should_deliver(&m, "pm:claude-code"));
    }

    #[test]
    fn addressed_delivered_to_sender() {
        let m = msg("dev:claude-code", Some(vec!["pm"]));
        assert!(should_deliver(&m, "dev:claude-code"));
    }

    #[test]
    fn addressed_not_delivered_to_non_recipient() {
        let m = msg("dev:claude-code", Some(vec!["pm"]));
        assert!(!should_deliver(&m, "designer:claude-code"));
    }

    #[test]
    fn group_addressed_delivered_to_all_named() {
        let m = msg("dev:claude-code", Some(vec!["pm", "designer"]));
        assert!(should_deliver(&m, "pm:claude-code"));
        assert!(should_deliver(&m, "designer:claude-code"));
        assert!(should_deliver(&m, "dev:human"));
        assert!(!should_deliver(&m, "ryan:human"));
    }

    #[test]
    fn to_all_is_broadcast() {
        let m = msg("dev:claude-code", Some(vec!["all"]));
        assert!(should_deliver(&m, "pm:claude-code"));
        assert!(should_deliver(&m, "ryan:human"));
        assert!(should_deliver(&m, "designer:claude-code"));
    }

    #[test]
    fn short_name_strips_tool_suffix() {
        assert_eq!(short_name("dev:claude-code"), "dev");
        assert_eq!(short_name("ryan:human"), "ryan");
        assert_eq!(short_name("bpmux"), "bpmux");
    }
}
