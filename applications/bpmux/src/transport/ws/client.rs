use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::SplitSink;

use super::client_handlers::{
    handle_pty_input, handle_pty_subscribe, handle_reaction_toggle, handle_reorder_iteration,
    handle_reorder_outcome_iteration, handle_reprioritize_outcome, handle_send,
    handle_set_outcome_horizon,
};
use super::hydrate::{
    handle_board_hydrate, handle_comments_hydrate, handle_history_hydrate, handle_history_page,
    handle_timeline_hydrate,
};
use super::{MAX_MESSAGE_SIZE, message_size_ok, send_json};
use crate::session;
use crate::transport::protocol::{ClientMessage, ServerMessage};

pub(crate) struct ClientCtx<'a> {
    pub session: &'a session::Session,
    pub store: &'a Arc<crate::store::Store>,
    pub pty_registry: &'a crate::room::LivePtyRegistry,
    pub pty_tx: &'a tokio::sync::mpsc::UnboundedSender<(String, Vec<u8>)>,
    pub tracker: &'a crate::transport::rest::TrackerHandle,
    pub orchestrator: &'a crate::server::OrchestratorHandle,
    pub reply_tx: &'a tokio::sync::mpsc::UnboundedSender<ServerMessage>,
}

pub(crate) async fn handle_client_message(
    text: &str,
    identity: &str,
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    pty_readers: &mut std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    if !message_size_ok(text) {
        crate::diag::log_diag(&format!(
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
            handle_send(ctx, identity, msg_text, to, addressee, message_type)?;
        }
        Ok(ClientMessage::Ping) => {
            send_json(tx, &ServerMessage::Pong).await?;
        }
        Ok(ClientMessage::Rename { name }) => {
            if super::is_reserved_display(&name) {
                send_json(
                    tx,
                    &ServerMessage::Error {
                        reason: format!(
                            "'{name}' is a reserved owner identity — an agent cannot present as the owner"
                        ),
                    },
                )
                .await?;
            } else {
                let tool = identity.split(':').nth(1).unwrap_or("ws");
                return Ok(Some(format!("{name}:{tool}")));
            }
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
            handle_pty_subscribe(ctx, terminal_id, pty_readers);
        }
        Ok(ClientMessage::PtyInput { terminal_id, data }) => {
            handle_pty_input(ctx, terminal_id, data);
        }
        Ok(ClientMessage::PtyResize {
            terminal_id,
            cols,
            rows,
        }) => {
            ctx.pty_registry.resize(&terminal_id, rows, cols);
        }
        Ok(ClientMessage::ReactionToggle {
            channel,
            message_key,
            emoji,
            user,
        }) => {
            handle_reaction_toggle(ctx, tx, channel, message_key, emoji, user).await?;
        }
        Ok(ClientMessage::CommentsHydrate { story_number }) => {
            handle_comments_hydrate(ctx, tx, story_number).await?;
        }
        Ok(ClientMessage::TimelineHydrate { story_number }) => {
            handle_timeline_hydrate(ctx, tx, story_number).await?;
        }
        Ok(ClientMessage::HistoryHydrate {
            chat_limit,
            activity_limit,
        }) => {
            handle_history_hydrate(ctx, tx, chat_limit, activity_limit).await?;
        }
        Ok(ClientMessage::HistoryPage {
            request_id,
            before,
            limit,
        }) => {
            handle_history_page(ctx, tx, request_id, before, limit).await?;
        }
        Ok(ClientMessage::BoardHydrate) => {
            handle_board_hydrate(ctx).await?;
        }
        Ok(ClientMessage::SetOutcomeHorizon {
            outcome_number,
            horizon,
        }) => {
            handle_set_outcome_horizon(ctx, tx, identity, outcome_number, horizon).await?;
        }
        Ok(ClientMessage::ReorderOutcomeIteration {
            outcome_number,
            iteration_number,
            position,
        }) => {
            handle_reorder_outcome_iteration(
                ctx,
                tx,
                identity,
                outcome_number,
                iteration_number,
                position,
            )
            .await?;
        }
        Ok(ClientMessage::ReprioritizeOutcome {
            outcome_number,
            position,
        }) => {
            handle_reprioritize_outcome(ctx, tx, identity, outcome_number, position).await?;
        }
        Ok(ClientMessage::ReorderIteration {
            iteration_number,
            position,
        }) => {
            handle_reorder_iteration(ctx, tx, identity, iteration_number, position).await?;
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
