use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use base64::Engine;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::session;
use crate::session::MessageType;
use crate::transport::protocol::{ClientMessage, ServerMessage};

mod client;
mod peer_lag;
mod roster;

pub(crate) use client::*;
pub use peer_lag::*;
pub(crate) use roster::*;

const MAX_MESSAGE_SIZE: usize = 65536;
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(session): State<Arc<session::Session>>,
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    axum::Extension(pty_registry): axum::Extension<Arc<crate::room::LivePtyRegistry>>,
    axum::Extension(routing): axum::Extension<Arc<crate::session::routing_identity::RoutingStore>>,
    axum::Extension(tracker): axum::Extension<crate::transport::rest::TrackerHandle>,
) -> Response {
    ws.on_upgrade(move |socket| {
        handle_ws(
            socket,
            session,
            orchestrator,
            pty_registry,
            routing,
            tracker,
        )
    })
}

async fn handle_ws(
    socket: WebSocket,
    session: Arc<session::Session>,
    orchestrator: Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    pty_registry: Arc<crate::room::LivePtyRegistry>,
    routing: Arc<crate::session::routing_identity::RoutingStore>,
    tracker: crate::transport::rest::TrackerHandle,
) {
    let (tx, rx) = socket.split();
    if let Err(e) = run_ws_session(
        tx,
        rx,
        session,
        orchestrator,
        pty_registry,
        routing,
        tracker,
    )
    .await
    {
        crate::session::diag::log_diag(&format!("ws: session error: {e}"));
    }
}

async fn run_ws_session(
    mut tx: SplitSink<WebSocket, WsMessage>,
    mut rx: SplitStream<WebSocket>,
    session: Arc<session::Session>,
    orchestrator: Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    pty_registry: Arc<crate::room::LivePtyRegistry>,
    routing: Arc<crate::session::routing_identity::RoutingStore>,
    tracker: crate::transport::rest::TrackerHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut identity, replay_cursor) = wait_for_identify(&mut tx, &mut rx).await?;
    let routing_session_id = routing.resolve(short_name(&identity)).into_iter().next();
    let mut roster_key = roster_key_for(&routing_session_id, &identity);
    session.peer_connected();
    if let Some(ref key) = roster_key {
        dispatch_roster_join(&orchestrator, key, &identity).await;
    }
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
    let snapshot_msg = build_state_snapshot_message(&orchestrator);
    send_json(&mut tx, &ServerMessage::from(snapshot_msg)).await?;
    let join_msg = system_announcement(&format!("{identity} joined"));
    session.broadcast(&join_msg)?;
    if let Some(cursor) = replay_cursor
        && let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&cursor)
        && let Ok(messages) = session.read_messages()
    {
        let cursor_ts = parsed.with_timezone(&chrono::Utc);
        for msg in messages {
            if msg.state_delta.is_some() && msg.ts > cursor_ts && should_deliver(&msg, &identity) {
                send_json(&mut tx, &ServerMessage::from(msg)).await?;
            }
        }
    }
    let (pty_tx, mut pty_rx) = tokio::sync::mpsc::unbounded_channel::<(String, Vec<u8>)>();
    let (reply_tx, mut reply_rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();
    let mut pty_readers: std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>> =
        std::collections::HashMap::new();
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
                            let lag_notice = ServerMessage::Error {
                                reason: format!("lagged: {n} messages dropped"),
                            };
                            let _ = send_json(&mut tx, &lag_notice).await;
                            let lag_broadcast = build_peer_lag_message(
                                &identity,
                                n,
                                chrono::Utc::now(),
                            );
                            if let Err(e) = session.broadcast(&lag_broadcast) {
                                session::diag::log_diag(&format!(
                                    "ws: peer-lag broadcast failed for {identity}: {e}"
                                ));
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = heartbeat.tick() => {
                    if tx.send(WsMessage::Ping(vec![].into())).await.is_err() {
                        break;
                    }
                }
                pty_chunk = pty_rx.recv() => {
                    if let Some((terminal_id, bytes)) = pty_chunk {
                        if bytes.is_empty() {
                            pty_readers.remove(&terminal_id);
                            send_json(&mut tx, &ServerMessage::PtyExit { terminal_id }).await?;
                        } else {
                            let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            send_json(&mut tx, &ServerMessage::PtyOutput { terminal_id, data }).await?;
                        }
                    }
                }
                reply = reply_rx.recv() => {
                    if let Some(msg) = reply {
                        send_json(&mut tx, &msg).await?;
                    }
                }
                ws_result = rx.next() => {
                    match ws_result {
                        Some(Ok(WsMessage::Text(text))) => {
                            let client_ctx = client::ClientCtx {
                                session: &session,
                                orchestrator: &orchestrator,
                                pty_registry: &pty_registry,
                                pty_tx: &pty_tx,
                                tracker: &tracker,
                                reply_tx: &reply_tx,
                            };
                            if let Some(new_identity) = handle_client_message(&text, &identity, &client_ctx, &mut tx, &mut pty_readers).await? {
                                let _ = session.broadcast(&system_announcement(&format!("{identity} left")));
                                identity = new_identity;
                                if let Some(ref session_id) = routing_session_id {
                                    routing.rename(session_id, short_name(&identity));
                                }
                                let new_key = roster_key_for(&routing_session_id, &identity);
                                if new_key != roster_key && let Some(ref old) = roster_key {
                                    dispatch_roster_leave(&orchestrator, old).await;
                                }
                                if let Some(ref key) = new_key {
                                    dispatch_roster_join(&orchestrator, key, &identity).await;
                                }
                                roster_key = new_key;
                                let _ = session.broadcast(&system_announcement(&format!("{identity} joined")));
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
    let leave_msg = system_announcement(&format!("{identity} left"));
    let _ = session.broadcast(&leave_msg);
    session.peer_disconnected();
    if let Some(ref key) = roster_key {
        dispatch_roster_leave(&orchestrator, key).await;
    }
    result
}

fn roster_key_for(routing_session_id: &Option<String>, identity: &str) -> Option<String> {
    if is_reserved_display(identity) {
        return None;
    }
    Some(
        routing_session_id
            .clone()
            .unwrap_or_else(|| identity.to_string()),
    )
}

async fn wait_for_identify(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    rx: &mut SplitStream<WebSocket>,
) -> Result<(String, Option<String>), Box<dyn std::error::Error + Send + Sync>> {
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = rx.next().await {
            if let WsMessage::Text(text) = msg {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Identify { name, tool, cursor }) => {
                        let tool_name = tool.unwrap_or_else(|| "ws".to_string());
                        return Ok((format!("{name}:{tool_name}"), cursor));
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

fn system_announcement(text: &str) -> session::Message {
    session::Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::System,
        ..session::Message::default()
    }
}

fn build_state_snapshot_message(
    orchestrator: &crate::orchestrator::Orchestrator<
        crate::board::adapters::github::GitHubSourceAdapter,
    >,
) -> session::Message {
    let snapshot = orchestrator.build_snapshot();
    session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: MessageType::StateSnapshot,
        state_snapshot: Some(snapshot),
        ..session::Message::default()
    }
}

pub(crate) fn short_name(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub(crate) fn should_deliver(msg: &session::Message, peer_identity: &str) -> bool {
    if matches!(
        role_from_short_name(short_name(peer_identity)),
        crate::orchestrator::stores::RosterRole::Owner
    ) {
        return msg.message_type != MessageType::Nudge;
    }
    let recipients = match &msg.to {
        None => return true,
        Some(r) => r,
    };
    if recipients
        .iter()
        .any(|r| r == crate::session::message::BROADCAST_ALL)
    {
        return true;
    }
    let peer_short = short_name(peer_identity);
    let sender_short = short_name(&msg.from);
    if peer_short == sender_short {
        return true;
    }
    recipients.iter().any(|r| short_name(r) == peer_short)
}

pub(crate) fn message_size_ok(text: &str) -> bool {
    text.len() <= MAX_MESSAGE_SIZE
}

async fn send_json(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    msg: &ServerMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string(msg)?;
    tx.send(WsMessage::Text(json.into())).await?;
    Ok(())
}
