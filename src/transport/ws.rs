use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::session;
use crate::transport::protocol::{ClientMessage, ServerMessage};
use crate::transport::state::ServerState;

const MAX_MESSAGE_SIZE: usize = 4096;

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<ServerState>>) -> Response {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: Arc<ServerState>) {
    let (tx, rx) = socket.split();

    if let Err(e) = run_ws_session(tx, rx, state).await {
        eprintln!("WebSocket session error: {e}");
    }
}

async fn run_ws_session(
    mut tx: SplitSink<WebSocket, WsMessage>,
    mut rx: SplitStream<WebSocket>,
    state: Arc<ServerState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Phase 1: Wait for identify (5 second timeout)
    let name = wait_for_identify(&mut tx, &mut rx).await?;
    let identity = format!("{name}:ws");

    // Subscribe to broadcast BEFORE sending welcome (avoid missing messages)
    let mut broadcast_rx = state.wake_tx.subscribe();

    // Send welcome
    send_json(
        &mut tx,
        &ServerMessage::Welcome {
            session_id: state.session_id.clone(),
        },
    )
    .await?;

    // Phase 2: Bidirectional message loop
    loop {
        tokio::select! {
            // Messages from broadcast -> push to WebSocket client
            broadcast_result = broadcast_rx.recv() => {
                match broadcast_result {
                    Ok(msg) => {
                        send_json(&mut tx, &ServerMessage::from(msg)).await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("WebSocket client {identity} lagged by {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // Messages from WebSocket client -> process
            ws_result = rx.next() => {
                match ws_result {
                    Some(Ok(WsMessage::Text(text))) => {
                        handle_client_message(
                            &text,
                            &identity,
                            &state,
                            &mut tx,
                        ).await?;
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Err(e)) => {
                        eprintln!("WebSocket read error for {identity}: {e}");
                        break;
                    }
                    _ => {} // Binary, Ping, Pong -- ignore
                }
            }
        }
    }

    Ok(())
}

async fn wait_for_identify(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    rx: &mut SplitStream<WebSocket>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = rx.next().await {
            if let WsMessage::Text(text) = msg {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Identify { name }) => return Ok(name),
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

async fn handle_client_message(
    text: &str,
    identity: &str,
    state: &ServerState,
    tx: &mut SplitSink<WebSocket, WsMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if text.len() > MAX_MESSAGE_SIZE {
        send_json(
            tx,
            &ServerMessage::Error {
                reason: "message too large".to_string(),
            },
        )
        .await?;
        return Ok(());
    }

    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::Send { text: msg_text }) => {
            let msg = session::Message {
                from: identity.to_string(),
                text: msg_text,
                ts: chrono::Utc::now(),
            };
            session::broadcast_message(&state.session_dir, &msg, &state.wake_tx)?;
        }
        Ok(ClientMessage::Ping) => {
            send_json(tx, &ServerMessage::Pong).await?;
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

    Ok(())
}

async fn send_json(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    msg: &ServerMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string(msg)?;
    tx.send(WsMessage::Text(json.into())).await?;
    Ok(())
}
