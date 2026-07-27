use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::SplitSink;

use protocol::ServerMessage;

use super::{send_json, should_deliver};
use message::MessageType;
use store::{AppAction, ClientConnection};

const REPLAY_SCAN_WINDOW: usize = 500;

pub(super) fn system_announcement(text: &str) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::System,
        ..message::Message::default()
    }
}

pub(super) async fn send_startup_frames(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    hub: &Arc<hub::Hub>,
    store: &Arc<store::Store>,
    identity: &str,
    routing_user_id: Option<&str>,
    is_owner: bool,
    replay_cursor: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    send_json(tx, &ServerMessage::Welcome {}).await?;
    store
        .dispatcher()
        .dispatch(AppAction::ClientConnected(ClientConnection {
            user_id: routing_user_id.map(String::from),
            identity: identity.to_string(),
        }));
    let join_msg = system_announcement(&format!("{identity} joined"));
    hub.broadcast(&join_msg)?;
    if let Some(cursor) = replay_cursor
        && let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&cursor)
        && let Ok(mut messages) =
            hub.read_recent(REPLAY_SCAN_WINDOW, 0, &|m| m.state_delta.is_some())
    {
        messages.reverse();
        let cursor_ts = parsed.with_timezone(&chrono::Utc);
        for msg in messages {
            if msg.state_delta.is_some()
                && msg.ts > cursor_ts
                && should_deliver(&msg, routing_user_id, identity, is_owner)
            {
                send_json(tx, &ServerMessage::from(msg)).await?;
            }
        }
    }
    Ok(())
}
