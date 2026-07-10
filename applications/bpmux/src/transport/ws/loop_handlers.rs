use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use base64::Engine;
use futures::stream::SplitSink;
use tokio::sync::broadcast;

use crate::hub;
use crate::transport::protocol::ServerMessage;

use super::{build_peer_lag_message, send_json, should_deliver};

pub(super) async fn handle_broadcast(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    result: Result<message::Message, broadcast::error::RecvError>,
    identity: &str,
    hub: &Arc<hub::Hub>,
    routing_user_id: Option<&str>,
    is_owner: bool,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    match result {
        Ok(msg) => {
            if should_deliver(&msg, routing_user_id, identity, is_owner) {
                send_json(tx, &ServerMessage::from(msg)).await?;
            }
        }
        Err(broadcast::error::RecvError::Lagged(n)) => {
            diag::log_diag(&format!("ws: client {identity} lagged by {n} messages"));
            let lag_notice = ServerMessage::Error {
                reason: format!("lagged: {n} messages dropped"),
            };
            let _ = send_json(tx, &lag_notice).await;
            let lag_broadcast = build_peer_lag_message(identity, n, chrono::Utc::now());
            if let Err(e) = hub.broadcast(&lag_broadcast) {
                diag::log_diag(&format!(
                    "ws: peer-lag broadcast failed for {identity}: {e}"
                ));
            }
        }
        Err(broadcast::error::RecvError::Closed) => return Ok(false),
    }
    Ok(true)
}

pub(super) async fn handle_pty_chunk(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    chunk: Option<(String, Vec<u8>)>,
    pty_registry: &Arc<crate::room::LivePtyRegistry>,
    pty_readers: &mut HashMap<String, Arc<AtomicBool>>,
    pty_modes: &mut HashMap<String, bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some((terminal_id, bytes)) = chunk else {
        return Ok(());
    };
    if bytes.is_empty() {
        pty_readers.remove(&terminal_id);
        pty_modes.remove(&terminal_id);
        send_json(tx, &ServerMessage::PtyExit { terminal_id }).await?;
        return Ok(());
    }
    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
    send_json(
        tx,
        &ServerMessage::PtyOutput {
            terminal_id: terminal_id.clone(),
            data,
        },
    )
    .await?;
    if let Some(bracketed_paste) = pty_registry.current_bracketed_paste(&terminal_id)
        && pty_modes.get(&terminal_id) != Some(&bracketed_paste)
    {
        pty_modes.insert(terminal_id.clone(), bracketed_paste);
        send_json(
            tx,
            &ServerMessage::PtyMode {
                terminal_id,
                bracketed_paste,
            },
        )
        .await?;
    }
    Ok(())
}
