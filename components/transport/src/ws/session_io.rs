use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};

use protocol::{ClientMessage, ServerMessage};

pub(super) async fn wait_for_identify(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    rx: &mut SplitStream<WebSocket>,
) -> Result<
    (String, Option<String>, Option<String>, Option<String>),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(msg)) = rx.next().await {
            let WsMessage::Text(text) = msg else {
                continue;
            };
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::Identify {
                    name,
                    tool,
                    cursor,
                    user_id,
                    owner_nonce,
                }) => return Ok(identity(name, tool, cursor, user_id, owner_nonce)),
                Ok(_) => reject(tx, "must identify first".to_string()).await?,
                Err(error) => reject(tx, format!("invalid message: {error}")).await?,
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

fn identity(
    name: String,
    tool: Option<String>,
    cursor: Option<String>,
    user_id: Option<String>,
    owner_nonce: Option<String>,
) -> (String, Option<String>, Option<String>, Option<String>) {
    let tool_name = tool.unwrap_or_else(|| "ws".to_string());
    (format!("{name}:{tool_name}"), cursor, user_id, owner_nonce)
}

async fn reject(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    reason: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    send_json(tx, &ServerMessage::Error { reason }).await
}

pub async fn send_json(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    msg: &ServerMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string(msg)?;
    tx.send(WsMessage::Text(json.into())).await?;
    Ok(())
}
