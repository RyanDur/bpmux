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
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

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
    let mut identity = format!("{name}:ws");
    session.peer_connected();
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
    let join_msg = session::Message {
        from: "bpmux".to_string(),
        text: format!("{identity} joined"),
        ts: chrono::Utc::now(),
        message_type: Some("system".to_string()),
    };
    session.broadcast(&join_msg)?;
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
                _ = heartbeat.tick() => {
                    if tx.send(WsMessage::Ping(vec![].into())).await.is_err() {
                        break;
                    }
                }
                ws_result = rx.next() => {
                    match ws_result {
                        Some(Ok(WsMessage::Text(text))) => {
                            if let Some(new_identity) = handle_client_message(&text, &identity, &session, &mut tx).await? {
                                identity = new_identity;
                            }
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
    let leave_msg = session::Message {
        from: "bpmux".to_string(),
        text: format!("{identity} left"),
        ts: chrono::Utc::now(),
        message_type: Some("system".to_string()),
    };
    let _ = session.broadcast(&leave_msg);
    session.peer_disconnected();
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

/// Returns Some(new_identity) if the client renamed itself.
async fn handle_client_message(
    text: &str,
    identity: &str,
    session: &session::Session,
    tx: &mut SplitSink<WebSocket, WsMessage>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    if text.len() > MAX_MESSAGE_SIZE {
        send_json(
            tx,
            &ServerMessage::Error {
                reason: "message too large".to_string(),
            },
        )
        .await?;
        return Ok(None);
    }
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::Send { text: msg_text }) => {
            let msg = session::Message {
                from: identity.to_string(),
                text: msg_text,
                ts: chrono::Utc::now(),
                message_type: None,
            };
            session.broadcast(&msg)?;
        }
        Ok(ClientMessage::Ping) => {
            send_json(tx, &ServerMessage::Pong).await?;
        }
        Ok(ClientMessage::Rename { name }) => {
            return Ok(Some(format!("{name}:ws")));
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
