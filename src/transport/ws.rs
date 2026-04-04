use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::session;
use crate::transport::protocol::{ClientMessage, ServerMessage};

const MAX_MESSAGE_SIZE: usize = 4096;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(session): State<Arc<session::Session>>,
) -> Response {
    ws.on_upgrade(|socket| handle_ws(socket, session))
}

async fn handle_ws(socket: WebSocket, session: Arc<session::Session>) {
    let (tx, rx) = socket.split();
    if let Err(e) = run_ws_session(tx, rx, session).await {
        eprintln!("WebSocket session error: {e}");
    }
}

async fn run_ws_session(
    mut tx: SplitSink<WebSocket, WsMessage>,
    mut rx: SplitStream<WebSocket>,
    session: Arc<session::Session>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let name = wait_for_identify(&mut tx, &mut rx).await?;
    let identity = format!("{name}:ws");
    let count = session.peer_connected();
    eprintln!("WebSocket peer {identity} connected ({count} total)");
    let mut broadcast_rx = session.subscribe();
    send_json(
        &mut tx,
        &ServerMessage::Welcome {
            session_id: session.id.clone(),
        },
    )
    .await?;
    let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
        loop {
            tokio::select! {
                broadcast_result = broadcast_rx.recv() => {
                    match broadcast_result {
                        Ok(msg) => { send_json(&mut tx, &ServerMessage::from(msg)).await?; }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!("WebSocket client {identity} lagged by {n} messages");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                ws_result = rx.next() => {
                    match ws_result {
                        Some(Ok(WsMessage::Text(text))) => {
                            handle_client_message(&text, &identity, &session, &mut tx).await?;
                        }
                        Some(Ok(WsMessage::Close(_))) | None => break,
                        Some(Err(e)) => { eprintln!("WebSocket read error for {identity}: {e}"); break; }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
    .await;
    let count = session.peer_disconnected();
    eprintln!("WebSocket peer {identity} disconnected ({count} remaining)");
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
    session: &session::Session,
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
            session.broadcast(&msg)?;
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
