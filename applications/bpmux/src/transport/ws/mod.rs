use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use base64::Engine;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::session;
use crate::transport::protocol::ServerMessage;

mod client;
mod client_handlers;
mod delivery;
mod hydrate;
mod peer_lag;
mod roster;
mod session_io;
mod snapshot;

pub(crate) use client::*;
pub(crate) use delivery::*;
pub use peer_lag::*;
pub(crate) use roster::*;
pub(crate) use session_io::send_json;
use session_io::wait_for_identify;
use snapshot::{build_state_snapshot_message, system_announcement};

const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

struct WsDeps {
    session: Arc<session::Session>,
    orchestrator:
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    pty_registry: Arc<crate::room::LivePtyRegistry>,
    routing: Arc<crate::session::routing_identity::RoutingStore>,
    tracker: crate::transport::rest::TrackerHandle,
    test_refresh: crate::server::TestRefreshHandle,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(session): State<Arc<session::Session>>,
    axum::Extension(orchestrator): axum::Extension<
        Arc<crate::orchestrator::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>>,
    >,
    axum::Extension(pty_registry): axum::Extension<Arc<crate::room::LivePtyRegistry>>,
    axum::Extension(routing): axum::Extension<Arc<crate::session::routing_identity::RoutingStore>>,
    axum::Extension(tracker): axum::Extension<crate::transport::rest::TrackerHandle>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
) -> Response {
    let deps = WsDeps {
        session,
        orchestrator,
        pty_registry,
        routing,
        tracker,
        test_refresh,
    };
    ws.on_upgrade(move |socket| handle_ws(socket, deps))
}

async fn handle_ws(socket: WebSocket, deps: WsDeps) {
    let (tx, rx) = socket.split();
    if let Err(e) = run_ws_session(tx, rx, deps).await {
        crate::session::diag::log_diag(&format!("ws: session error: {e}"));
    }
}

async fn run_ws_session(
    mut tx: SplitSink<WebSocket, WsMessage>,
    mut rx: SplitStream<WebSocket>,
    deps: WsDeps,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let WsDeps {
        session,
        orchestrator,
        pty_registry,
        routing,
        tracker,
        test_refresh,
    } = deps;
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
                                test_refresh: &test_refresh,
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
