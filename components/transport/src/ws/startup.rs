use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::SplitSink;

use protocol::ServerMessage;

use super::snapshot::{build_state_snapshot_message, system_announcement};
use super::{send_json, should_deliver};

const REPLAY_SCAN_WINDOW: usize = 500;

#[allow(clippy::too_many_arguments)]
pub(super) async fn send_startup_frames(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    hub: &Arc<hub::Hub>,
    store: &Arc<store::Store>,
    orchestrator: &sync::OrchestratorHandle,
    identity: &str,
    routing_user_id: Option<&str>,
    is_owner: bool,
    replay_cursor: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    send_json(tx, &ServerMessage::Welcome {}).await?;
    let snapshot_msg = build_state_snapshot_message(store);
    send_json(tx, &ServerMessage::from(snapshot_msg)).await?;
    if orchestrator.as_ref().is_some() {
        let select = store.selector();
        let stories = serde_json::Value::Array(crate::rest::board_stories(&select));
        send_json(
            tx,
            &ServerMessage::from(crate::rest::stories_message(stories)),
        )
        .await?;
        let epics = serde_json::Value::Array(crate::rest::board_epics(&select));
        send_json(tx, &ServerMessage::from(crate::rest::epics_message(epics))).await?;
        let outcomes = serde_json::Value::Array(crate::rest::board_outcomes(&select));
        send_json(
            tx,
            &ServerMessage::from(crate::rest::outcomes_message(outcomes)),
        )
        .await?;
        let iterations = serde_json::Value::Array(crate::rest::board_iterations(&select));
        send_json(
            tx,
            &ServerMessage::from(crate::rest::iterations_message(iterations)),
        )
        .await?;
        let roadmap = serde_json::Value::Array(crate::rest::board_roadmap(&select));
        send_json(
            tx,
            &ServerMessage::from(crate::rest::roadmap_message(roadmap)),
        )
        .await?;
    }
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
