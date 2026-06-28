use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::SplitSink;

use super::client::ClientCtx;
use super::send_json;
use crate::message::MessageType;
use crate::transport::protocol::ServerMessage;

pub(super) async fn handle_comments_hydrate(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    story_number: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match ctx.tracker.clone() {
        Some(tracker) => {
            let reply_tx = ctx.reply_tx.clone();
            tokio::spawn(async move {
                let reply = match tracker.list_story_comments(story_number).await {
                    Ok(comments) => ServerMessage::CommentsHydrated {
                        story_number,
                        comments,
                    },
                    Err(e) => ServerMessage::Error {
                        reason: format!("list comments failed: {e}"),
                    },
                };
                let _ = reply_tx.send(reply);
            });
        }
        None => {
            send_json(
                tx,
                &ServerMessage::Error {
                    reason: "tracker not configured".to_string(),
                },
            )
            .await?;
        }
    }
    Ok(())
}

pub(super) async fn handle_history_hydrate(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    chat_limit: usize,
    activity_limit: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let chat = ctx
        .session
        .read_recent(chat_limit, 0, &|m: &crate::message::Message| m.is_chat());
    let activity = ctx
        .session
        .read_recent(activity_limit, 0, &|m: &crate::message::Message| {
            m.message_type == MessageType::Activity
        });
    let reply = match (chat, activity) {
        (Ok(mut combined), Ok(activity)) => {
            combined.extend(activity);
            combined.sort_by_key(|m| m.ts);
            let serialized: Vec<serde_json::Value> = combined
                .iter()
                .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
                .collect();
            ServerMessage::HistoryHydrated {
                messages: serialized,
            }
        }
        (Err(e), _) | (_, Err(e)) => ServerMessage::Error {
            reason: format!("read history failed: {e}"),
        },
    };
    send_json(tx, &reply).await?;
    Ok(())
}

pub(super) async fn handle_board_hydrate(
    ctx: &ClientCtx<'_>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_refresh = ctx.test_refresh.clone();
    let tracker = ctx.tracker.clone();
    let reply_tx = ctx.reply_tx.clone();
    tokio::spawn(async move {
        let stories: Option<Vec<serde_json::Value>> = if let Some(refresh) = test_refresh.as_ref() {
            if refresh.all_stories().is_empty() {
                let _ = refresh.refresh_all().await;
            }
            Some(crate::transport::rest::ordered_board_stories(refresh))
        } else if let Some(tracker) = tracker.as_ref() {
            tracker.fetch_board_stories().await.ok().map(
                crate::product_management::board::ready_order_label::sort_board_ready_by_membership,
            )
        } else {
            None
        };
        if let Some(stories) = stories {
            let msg =
                crate::transport::rest::board_snapshot_message(serde_json::Value::Array(stories));
            let _ = reply_tx.send(ServerMessage::from(msg));
        }
        if let Some(refresh) = test_refresh.as_ref() {
            let outcomes =
                serde_json::Value::Array(crate::transport::rest::board_outcomes(refresh));
            let _ = reply_tx.send(ServerMessage::from(
                crate::transport::rest::outcomes_message(outcomes),
            ));
            let iterations =
                serde_json::Value::Array(crate::transport::rest::board_iterations(refresh));
            let _ = reply_tx.send(ServerMessage::from(
                crate::transport::rest::iterations_message(iterations),
            ));
        }
    });
    Ok(())
}

pub(super) async fn handle_history_page(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    request_id: u64,
    before: String,
    limit: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let messages: Vec<serde_json::Value> = match before.parse::<chrono::DateTime<chrono::Utc>>() {
        Ok(before_ts) => match ctx
            .session
            .read_recent(limit, 0, &|m: &crate::message::Message| {
                m.is_chat() && m.ts < before_ts
            }) {
            Ok(mut newest_first) => {
                newest_first.reverse();
                newest_first
                    .iter()
                    .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
                    .collect()
            }
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };
    send_json(
        tx,
        &ServerMessage::HistoryPageLoaded {
            request_id,
            messages,
        },
    )
    .await?;
    Ok(())
}

pub(super) async fn handle_timeline_hydrate(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    story_number: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match ctx.tracker.clone() {
        Some(tracker) => {
            let reply_tx = ctx.reply_tx.clone();
            tokio::spawn(async move {
                let reply = match tracker.list_story_timeline(story_number).await {
                    Ok(events) => ServerMessage::TimelineHydrated {
                        story_number,
                        events,
                    },
                    Err(e) => ServerMessage::Error {
                        reason: format!("list timeline failed: {e}"),
                    },
                };
                let _ = reply_tx.send(reply);
            });
        }
        None => {
            send_json(
                tx,
                &ServerMessage::Error {
                    reason: "tracker not configured".to_string(),
                },
            )
            .await?;
        }
    }
    Ok(())
}
