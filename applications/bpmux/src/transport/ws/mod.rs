use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};

use crate::routing::short_name;
use crate::session;
use crate::transport::protocol::ServerMessage;

mod client;
mod client_handlers;
mod delivery;
mod handler;
mod hydrate;
mod loop_handlers;
mod peer_lag;
mod roadmap_write_handlers;
mod roster;
mod session_io;
mod snapshot;
mod startup;

#[cfg(test)]
mod roster_projection;

pub(crate) use client::*;
pub(crate) use delivery::*;
pub use handler::ws_handler;
pub use peer_lag::*;
pub(crate) use roster::*;
pub(crate) use session_io::send_json;
use session_io::wait_for_identify;
use snapshot::system_announcement;

const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

pub(super) struct WsDeps {
    session: Arc<session::Session>,
    store: Arc<crate::store::Store>,
    dispatch: crate::store::Dispatch,
    pty_registry: Arc<crate::room::LivePtyRegistry>,
    routing: Arc<crate::routing::RoutingStore>,
    tracker: crate::transport::rest::TrackerHandle,
    orchestrator: crate::sync::OrchestratorHandle,
}

pub(super) async fn run_ws_session(
    mut tx: SplitSink<WebSocket, WsMessage>,
    mut rx: SplitStream<WebSocket>,
    deps: WsDeps,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let WsDeps {
        session,
        store,
        dispatch,
        pty_registry,
        routing,
        tracker,
        orchestrator,
    } = deps;
    let (mut identity, replay_cursor) = wait_for_identify(&mut tx, &mut rx).await?;
    let routing_session_id = routing.resolve(short_name(&identity)).into_iter().next();
    let mut roster_key = roster_key_for(&routing_session_id, &identity);
    session.peer_connected();
    if let Some(ref key) = roster_key {
        dispatch_roster_join(&store, key, &identity).await;
    }
    let mut broadcast_rx = session.subscribe();
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.tick().await;
    startup::send_startup_frames(
        &mut tx,
        &session,
        &store,
        &orchestrator,
        &identity,
        replay_cursor,
    )
    .await?;
    let (pty_tx, mut pty_rx) = tokio::sync::mpsc::unbounded_channel::<(String, Vec<u8>)>();
    let (reply_tx, mut reply_rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();
    let mut pty_readers: std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>> =
        std::collections::HashMap::new();
    let mut pty_modes: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
        loop {
            tokio::select! {
                broadcast_result = broadcast_rx.recv() => {
                    if !loop_handlers::handle_broadcast(&mut tx, broadcast_result, &identity, &session).await? {
                        break;
                    }
                }
                _ = heartbeat.tick() => {
                    if tx.send(WsMessage::Ping(vec![].into())).await.is_err() {
                        break;
                    }
                }
                pty_chunk = pty_rx.recv() => {
                    loop_handlers::handle_pty_chunk(&mut tx, pty_chunk, &pty_registry, &mut pty_readers, &mut pty_modes).await?;
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
                                store: &store,
                                dispatch: &dispatch,
                                pty_registry: &pty_registry,
                                pty_tx: &pty_tx,
                                tracker: &tracker,
                                orchestrator: &orchestrator,
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
                                    dispatch_roster_leave(&store, old).await;
                                }
                                if let Some(ref key) = new_key {
                                    dispatch_roster_join(&store, key, &identity).await;
                                }
                                roster_key = new_key;
                                let _ = session.broadcast(&system_announcement(&format!("{identity} joined")));
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) | None => break,
                        Some(Err(e)) => {
                            crate::diag::log_diag(&format!(
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
        dispatch_roster_leave(&store, key).await;
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
