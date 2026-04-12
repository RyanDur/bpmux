use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::session;
use crate::session::MessageType;
use crate::transport::protocol::{ClientMessage, ServerMessage};

/// Maximum accepted size for an inbound client-to-server WebSocket text
/// frame. Messages larger than this are rejected with a `ServerMessage::Error`
/// reply and NOT broadcast.
///
/// **History** (#tbd — 2026-04-11): originally set to 4096. During a
/// nav/drive pair-programming experiment on 2026-04-10 evening, coordination
/// messages (detailed test steps, multi-paragraph instructions, retrospectives)
/// repeatedly exceeded the 4096 limit and were silently dropped from the
/// sender's perspective. Silent because `handle_client_message` returned
/// `Ok(None)` after the size rejection — the broadcast never happened, and
/// the sender's `ws_send` (see `src/bridge/tools.rs::ws_send`) only checks
/// that the frame left the socket, not that the server accepted it.
///
/// Raised to 65536 (64 KB) — comfortably larger than any legitimate
/// coordination message bpmux carries (typical: ~1 KB, largest observed:
/// ~5 KB for session retros), still bounded against DoS. If a future use
/// case legitimately needs bigger messages (code dumps, diff pastes, full
/// docs), we should introduce chunked messaging rather than raise this
/// ceiling further — 64 KB is a generous floor, not a design target.
const MAX_MESSAGE_SIZE: usize = 65536;
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
        crate::session::diag::log_diag(&format!("ws: session error: {e}"));
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
    let join_msg = session::Message {
        from: "bpmux".to_string(),
        text: format!("{identity} joined"),
        ts: chrono::Utc::now(),
        message_type: MessageType::System,
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
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
                            crate::session::diag::log_diag(&format!(
                                "ws: client {identity} lagged by {n} messages"
                            ));
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
                                let leave = session::Message {
                                    from: "bpmux".to_string(),
                                    text: format!("{identity} left"),
                                    ts: chrono::Utc::now(),
                                    message_type: MessageType::System,
                                    to: None,
                                    addressee: None,
                                    explicit_privacy: false,
                                    multi_depth: None,
                                };
                                let _ = session.broadcast(&leave);
                                identity = new_identity;
                                let join = session::Message {
                                    from: "bpmux".to_string(),
                                    text: format!("{identity} joined"),
                                    ts: chrono::Utc::now(),
                                    message_type: MessageType::System,
                                    to: None,
                                    addressee: None,
                                    explicit_privacy: false,
                                    multi_depth: None,
                                };
                                let _ = session.broadcast(&join);
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) | None => break,
                        Some(Err(e)) => {
                            crate::session::diag::log_diag(&format!(
                                "ws: read error for {identity}: {e}"
                            ));
                            break;
                        }
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
        message_type: MessageType::System,
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
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

pub(crate) fn short_name(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub(crate) fn should_deliver(msg: &session::Message, peer_identity: &str) -> bool {
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

/// Pure size check extracted for testability. Returns `true` if the
/// message body fits within the accepted inbound frame budget; callers
/// reject oversized messages before broadcasting.
///
/// Extracted on 2026-04-11 as part of the silent-drop fix so a unit test
/// can regression-guard `MAX_MESSAGE_SIZE` without having to stand up a
/// real WebSocket. If someone lowers the constant back toward its old
/// (4096) value and breaks legitimate coordination messages, the test
/// will fail at build time instead of silently reintroducing the bug.
pub(crate) fn message_size_ok(text: &str) -> bool {
    text.len() <= MAX_MESSAGE_SIZE
}

async fn handle_client_message(
    text: &str,
    identity: &str,
    session: &session::Session,
    tx: &mut SplitSink<WebSocket, WsMessage>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    if !message_size_ok(text) {
        // Log the drop so oversized messages become visible in the diag
        // log. Without this, drops were the silent-failure mode that
        // caused the 2026-04-10 nav/drive experiment to stall — the
        // server rejected, the client's ws_send returned success, nobody
        // saw the failure. The diag log (gated on `BPMUX_LOG` env var)
        // gives post-hoc evidence even if the error reply is ignored.
        crate::session::diag::log_diag(&format!(
            "ws: rejected oversized message from {identity}: {} bytes (max {MAX_MESSAGE_SIZE})",
            text.len()
        ));
        send_json(
            tx,
            &ServerMessage::Error {
                reason: format!(
                    "message too large: {} bytes exceeds limit of {MAX_MESSAGE_SIZE}",
                    text.len()
                ),
            },
        )
        .await?;
        return Ok(None);
    }
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::Send {
            text: msg_text,
            to,
            addressee,
            message_type,
        }) => {
            let msg = session::Message {
                from: identity.to_string(),
                text: msg_text,
                ts: chrono::Utc::now(),
                message_type: MessageType::from(message_type.as_deref()),
                to,
                addressee,
                explicit_privacy: false,
                multi_depth: None,
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
