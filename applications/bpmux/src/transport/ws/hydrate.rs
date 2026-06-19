use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::SplitSink;

use super::client::ClientCtx;
use super::send_json;
use crate::session;
use crate::session::MessageType;
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
    let reply = match ctx.session.read_messages() {
        Ok(messages) => {
            let mut combined: Vec<session::Message> = messages
                .iter()
                .filter(|m| m.is_chat())
                .rev()
                .take(chat_limit)
                .cloned()
                .collect();
            combined.extend(
                messages
                    .iter()
                    .filter(|m| m.message_type == MessageType::Activity)
                    .rev()
                    .take(activity_limit)
                    .cloned(),
            );
            combined.sort_by_key(|m| m.ts);
            let serialized: Vec<serde_json::Value> = combined
                .iter()
                .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
                .collect();
            ServerMessage::HistoryHydrated {
                messages: serialized,
            }
        }
        Err(e) => ServerMessage::Error {
            reason: format!("read history failed: {e}"),
        },
    };
    send_json(tx, &reply).await?;
    Ok(())
}

pub(super) async fn handle_history_page(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    request_id: u64,
    before: String,
    limit: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let messages: Vec<serde_json::Value> = match (
        ctx.session.read_messages(),
        before.parse::<chrono::DateTime<chrono::Utc>>(),
    ) {
        (Ok(all), Ok(before_ts)) => {
            let older: Vec<session::Message> = all
                .into_iter()
                .filter(|m| m.is_chat() && m.ts < before_ts)
                .collect();
            let start = older.len().saturating_sub(limit);
            older
                .iter()
                .skip(start)
                .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
                .collect()
        }
        _ => Vec::new(),
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
