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
) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, session, orchestrator))
}

async fn handle_ws(
    socket: WebSocket,
    session: Arc<session::Session>,
    orchestrator: Arc<
        crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
) {
    let (tx, rx) = socket.split();
    if let Err(e) = run_ws_session(tx, rx, session, orchestrator).await {
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
                ws_result = rx.next() => {
                    match ws_result {
                        Some(Ok(WsMessage::Text(text))) => {
                            if let Some(new_identity) = handle_client_message(&text, &identity, &session, &mut tx).await? {
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
    recipients.iter().any(|r| r == peer_short)
}

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
