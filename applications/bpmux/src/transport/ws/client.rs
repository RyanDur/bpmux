use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use base64::Engine;
use futures::stream::SplitSink;

use super::{MAX_MESSAGE_SIZE, message_size_ok, send_json};
use crate::session;
use crate::session::MessageType;
use crate::transport::protocol::{ClientMessage, ServerMessage};

pub(crate) async fn handle_client_message(
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
            let broadcast = match &msg.to {
                None => true,
                Some(r) => r
                    .iter()
                    .any(|x| x == crate::session::message::BROADCAST_ALL),
            };
            if broadcast && msg.is_chat() {
                crate::room::inject::deliver_broadcast_to_terminals(
                    pty_registry,
                    crate::transport::ws::short_name(&msg.from),
                    &msg.text,
                );
            }
        }
        Ok(ClientMessage::BoardInvalidate) => {
            session.broadcast(&board_invalidated_state_delta())?;
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
