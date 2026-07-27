use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};

use protocol::ServerMessage;
use user_routing::short_name;

pub mod client;
pub mod client_handlers;
mod delivery;
mod handler;
mod hydrate;
mod identify;
mod loop_handlers;
mod peer_lag;
mod roadmap_write_handlers;
pub mod roster;
mod session_io;
mod startup;

pub use client::*;
pub use delivery::*;
pub use handler::ws_handler;
use identify::{resolve_routing_user_id, roster_key_for};
pub use peer_lag::*;
pub use roster::*;
pub use session_io::send_json;
use session_io::wait_for_identify;
use startup::system_announcement;

const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

pub(super) struct WsDeps {
    hub: Arc<hub::Hub>,
    store: Arc<store::Store>,
    dispatch: store::Dispatch,
    pty_registry: Arc<room::LivePtyRegistry>,
    cell: crate::rest::shared::TrackerCell,
    orchestrator: sync::OrchestratorHandle,
}

pub(super) async fn run_ws_session(
    mut tx: SplitSink<WebSocket, WsMessage>,
    mut rx: SplitStream<WebSocket>,
    deps: WsDeps,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let WsDeps {
        hub,
        store,
        dispatch,
        pty_registry,
        cell,
        orchestrator,
    } = deps;
    let (mut identity, replay_cursor, identify_user_id, identify_owner_nonce) =
        wait_for_identify(&mut tx, &mut rx).await?;
    let routing_user_id = resolve_routing_user_id(&store, &dispatch, &identity, identify_user_id);
    let claimed_owner_nonce = store
        .selector()
        .select(|state| owner_session::claimed_nonce(&state.owner_session));
    let is_owner = owner_session::resolve_owner_mark(
        claimed_owner_nonce.as_deref(),
        identify_owner_nonce.as_deref(),
        &identity,
    );
    let mut roster_key = roster_key_for(&routing_user_id, is_owner);
    hub.peer_connected();
    if let Some(ref key) = roster_key {
        dispatch_roster_join(&store, key, &identity).await;
    }
    let mut broadcast_rx = hub.subscribe();
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.tick().await;
    startup::send_startup_frames(
        &mut tx,
        &hub,
        &store,
        &identity,
        routing_user_id.as_deref(),
        is_owner,
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
                    if !loop_handlers::handle_broadcast(&mut tx, broadcast_result, &identity, &hub, routing_user_id.as_deref(), is_owner).await? {
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
                            let tracker: crate::rest::TrackerHandle = cell.current();
                            let client_ctx = client::ClientCtx {
                                hub: &hub,
                                store: &store,
                                dispatch: &dispatch,
                                pty_registry: &pty_registry,
                                pty_tx: &pty_tx,
                                tracker: &tracker,
                                orchestrator: &orchestrator,
                                reply_tx: &reply_tx,
                                routing_user_id: &routing_user_id,
                            };
                            if let Some(new_identity) = handle_client_message(&text, &identity, &client_ctx, &mut tx, &mut pty_readers).await? {
                                let _ = hub.broadcast(&system_announcement(&format!("{identity} left")));
                                identity = new_identity;
                                if let Some(ref user_id) = routing_user_id {
                                    dispatch.dispatch(
                                        store::routing::RoutingAction::Renamed {
                                            user_id: user_id.clone(),
                                            display: short_name(&identity).to_string(),
                                        }
                                        .into(),
                                    );
                                }
                                let new_key = roster_key_for(&routing_user_id, is_owner);
                                if new_key != roster_key && let Some(ref old) = roster_key {
                                    dispatch_roster_leave(&store, old).await;
                                }
                                if let Some(ref key) = new_key {
                                    dispatch_roster_join(&store, key, &identity).await;
                                }
                                roster_key = new_key;
                                let _ = hub.broadcast(&system_announcement(&format!("{identity} joined")));
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) | None => break,
                        Some(Err(e)) => {
                            diag::log_diag(&format!(
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
    let _ = hub.broadcast(&leave_msg);
    hub.peer_disconnected();
    if let Some(ref key) = roster_key {
        dispatch_roster_leave(&store, key).await;
    }
    result
}
