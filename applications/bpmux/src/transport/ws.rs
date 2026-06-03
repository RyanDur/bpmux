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

const MAX_MESSAGE_SIZE: usize = 65536;
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

pub const PEER_LAG_MARKER: &str = "__BPMUX_PEER_LAG__";

pub fn build_peer_lag_message(
    peer: &str,
    count: u64,
    ts: chrono::DateTime<chrono::Utc>,
) -> session::Message {
    let payload = serde_json::json!({ "peer": peer, "count": count });
    session::Message {
        from: "bpmux".to_string(),
        text: format!("{PEER_LAG_MARKER}:{payload}"),
        ts,
        message_type: MessageType::System,
        ..session::Message::default()
    }
}

pub fn parse_peer_lag_payload(text: &str) -> Option<(String, u64)> {
    let rest = text.strip_prefix(PEER_LAG_MARKER)?.strip_prefix(':')?;
    let value: serde_json::Value = serde_json::from_str(rest).ok()?;
    let peer = value.get("peer")?.as_str()?.to_string();
    let count = value.get("count")?.as_u64()?;
    Some((peer, count))
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(session): State<Arc<session::Session>>,
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    axum::Extension(pty_registry): axum::Extension<Arc<crate::room::LivePtyRegistry>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, session, orchestrator, pty_registry))
}

async fn handle_ws(
    socket: WebSocket,
    session: Arc<session::Session>,
    orchestrator: Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    pty_registry: Arc<crate::room::LivePtyRegistry>,
) {
    let (tx, rx) = socket.split();
    if let Err(e) = run_ws_session(tx, rx, session, orchestrator, pty_registry).await {
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut identity = wait_for_identify(&mut tx, &mut rx).await?;
    session.peer_connected();
    session.roster_join(&identity);
    dispatch_roster_join(&orchestrator, &identity).await;
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
    let (pty_tx, mut pty_rx) = tokio::sync::mpsc::unbounded_channel::<(String, Vec<u8>)>();
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
                ws_result = rx.next() => {
                    match ws_result {
                        Some(Ok(WsMessage::Text(text))) => {
                            if let Some(new_identity) = handle_client_message(&text, &identity, &session, &mut tx, &pty_registry, &pty_tx, &mut pty_readers).await? {
                                let _ = session.broadcast(&system_announcement(&format!("{identity} left")));
                                session.roster_leave(&identity);
                                dispatch_roster_leave(&orchestrator, &identity).await;
                                identity = new_identity;
                                session.roster_join(&identity);
                                dispatch_roster_join(&orchestrator, &identity).await;
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
    session.roster_leave(&identity);
    dispatch_roster_leave(&orchestrator, &identity).await;
    result
}

pub(crate) fn role_from_short_name(short: &str) -> crate::orchestrator::stores::RosterRole {
    use crate::orchestrator::stores::RosterRole;
    let workflow = crate::workflow::current_workflow();
    let purpose = workflow
        .role
        .iter()
        .find(|r| r.id == short)
        .and_then(|r| r.purposes.first().map(String::as_str));
    match purpose {
        Some("designer") => RosterRole::Designer,
        Some("dev") => RosterRole::Dev,
        Some("pm") => RosterRole::Pm,
        Some("owner") => RosterRole::Owner,
        _ => RosterRole::Other,
    }
}

pub(crate) async fn dispatch_roster_join(
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    identity: &str,
) {
    use crate::orchestrator::OrchestratorAction;
    use crate::orchestrator::stores::{Presence, RosterAction};
    let short = short_name(identity).to_string();
    let role = role_from_short_name(&short);
    orchestrator
        .dispatch(OrchestratorAction::Roster(
            RosterAction::RosterMemberUpserted {
                name: short,
                identity: identity.to_string(),
                role,
                presence: Presence::Online,
            },
        ))
        .await;
}

pub(crate) async fn dispatch_roster_leave(
    orchestrator: &Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
    identity: &str,
) {
    use crate::orchestrator::OrchestratorAction;
    use crate::orchestrator::stores::RosterAction;
    let short = short_name(identity).to_string();
    orchestrator
        .dispatch(OrchestratorAction::Roster(
            RosterAction::RosterMemberRemoved { name: short },
        ))
        .await;
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

async fn handle_client_message(
    text: &str,
    identity: &str,
    session: &session::Session,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    pty_registry: &crate::room::LivePtyRegistry,
    pty_tx: &tokio::sync::mpsc::UnboundedSender<(String, Vec<u8>)>,
    pty_readers: &mut std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    if !message_size_ok(text) {
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
            let explicit_privacy = to.is_some();
            let is_board_invalidate = msg_text == crate::bridge::BOARD_INVALIDATE_MARKER;
            let msg = session::Message {
                from: identity.to_string(),
                text: msg_text,
                ts: chrono::Utc::now(),
                message_type: MessageType::from(message_type.as_deref()),
                to,
                addressee,
                explicit_privacy,
                ..session::Message::default()
            };
            session.broadcast(&msg)?;
            if is_board_invalidate {
                session.broadcast(&board_invalidated_state_delta())?;
            }
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
        Ok(ClientMessage::PtySubscribe { terminal_id }) => {
            let already_streaming = pty_readers
                .get(&terminal_id)
                .is_some_and(|stop| !stop.load(std::sync::atomic::Ordering::Relaxed));
            if !already_streaming && let Some(Ok(reader)) = pty_registry.reader_for(&terminal_id) {
                let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
                pty_readers.insert(terminal_id.clone(), stop.clone());
                let sink = pty_tx.clone();
                std::thread::spawn(move || {
                    use std::io::Read;
                    let mut reader = reader;
                    let mut buf = [0u8; 4096];
                    loop {
                        if stop.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        match reader.read(&mut buf) {
                            Ok(0) => {
                                let _ = sink.send((terminal_id.clone(), Vec::new()));
                                break;
                            }
                            Ok(n) => {
                                if sink.send((terminal_id.clone(), buf[..n].to_vec())).is_err() {
                                    break;
                                }
                            }
                            Err(_) => {
                                let _ = sink.send((terminal_id.clone(), Vec::new()));
                                break;
                            }
                        }
                    }
                });
            }
        }
        Ok(ClientMessage::PtyInput { terminal_id, data }) => {
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&data) {
                pty_registry.write_to(&terminal_id, &bytes);
            }
        }
        Ok(ClientMessage::PtyResize {
            terminal_id,
            cols,
            rows,
        }) => {
            pty_registry.resize(&terminal_id, rows, cols);
        }
        Ok(ClientMessage::MarkReceipt { message_id, status }) => {
            let recipient = short_name(identity).to_string();
            session.broadcast(&message_status_state_delta(message_id, recipient, status))?;
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

fn message_status_state_delta(
    message_id: String,
    recipient: String,
    status: crate::session::message::DeliveryStatus,
) -> session::Message {
    session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(crate::session::message::StateDeltaPayload::MessageStatus {
            message_id,
            recipient,
            status,
        }),
        ..session::Message::default()
    }
}

fn board_invalidated_state_delta() -> session::Message {
    session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(
            crate::session::message::StateDeltaPayload::BoardInvalidated {
                reason: "board changed".to_string(),
            },
        ),
        ..session::Message::default()
    }
}

async fn send_json(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    msg: &ServerMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string(msg)?;
    tx.send(WsMessage::Text(json.into())).await?;
    Ok(())
}
