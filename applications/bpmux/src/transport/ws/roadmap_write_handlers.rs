use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::SplitSink;

use super::client::ClientCtx;
use super::send_json;
use crate::product_management::iteration::IterationId;
use crate::product_management::outcome::OutcomeId;
use crate::transport::protocol::ServerMessage;

pub(super) async fn handle_set_outcome_horizon(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    identity: &str,
    outcome_id: OutcomeId,
    horizon: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "outcome_id".to_string(),
        serde_json::json!(outcome_id.value()),
    );
    args.insert("to".to_string(), serde_json::json!(horizon));
    let role = crate::bridge::tools_stories::role_from_name(identity);
    if let Err(e) = crate::bridge::dispatch_reprioritize_outcome(
        args,
        identity,
        role,
        ctx.routing_user_id.as_deref(),
        ctx.orchestrator.as_ref(),
        ctx.dispatch,
    )
    .await
    {
        send_json(
            tx,
            &ServerMessage::Error {
                reason: format!("set outcome horizon failed: {e}"),
            },
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn handle_reorder_outcome_iteration(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    identity: &str,
    outcome_id: OutcomeId,
    iteration_id: IterationId,
    position: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "outcome_id".to_string(),
        serde_json::json!(outcome_id.value()),
    );
    args.insert(
        "iteration_id".to_string(),
        serde_json::json!(iteration_id.value()),
    );
    args.insert("position".to_string(), serde_json::json!(position));
    let role = crate::bridge::tools_stories::role_from_name(identity);
    if let Err(e) = crate::bridge::dispatch_reorder_outcome_iteration(
        args,
        role,
        ctx.orchestrator.as_ref(),
        ctx.dispatch,
    )
    .await
    {
        send_json(
            tx,
            &ServerMessage::Error {
                reason: format!("reorder outcome iteration failed: {e}"),
            },
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn handle_reorder_iteration(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    identity: &str,
    iteration_id: IterationId,
    position: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "iteration_id".to_string(),
        serde_json::json!(iteration_id.value()),
    );
    args.insert("position".to_string(), serde_json::json!(position));
    let role = crate::bridge::tools_stories::role_from_name(identity);
    if let Err(e) = crate::bridge::dispatch_reorder_iteration(
        args,
        identity,
        role,
        ctx.routing_user_id.as_deref(),
        ctx.orchestrator.as_ref(),
        ctx.dispatch,
    )
    .await
    {
        send_json(
            tx,
            &ServerMessage::Error {
                reason: format!("reorder iteration failed: {e}"),
            },
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn handle_add_story_to_iteration(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    identity: &str,
    iteration_id: IterationId,
    story_id: u64,
    position: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "iteration_id".to_string(),
        serde_json::json!(iteration_id.value()),
    );
    args.insert("story_id".to_string(), serde_json::json!(story_id));
    args.insert("position".to_string(), serde_json::json!(position));
    let role = crate::bridge::tools_stories::role_from_name(identity);
    if let Err(e) = crate::bridge::dispatch_add_story_to_iteration(
        args,
        role,
        ctx.orchestrator.as_ref(),
        ctx.dispatch,
    )
    .await
    {
        send_json(
            tx,
            &ServerMessage::Error {
                reason: format!("add story to iteration failed: {e}"),
            },
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn handle_reprioritize_outcome(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    identity: &str,
    outcome_id: OutcomeId,
    position: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "outcome_id".to_string(),
        serde_json::json!(outcome_id.value()),
    );
    args.insert("position".to_string(), serde_json::json!(position));
    let role = crate::bridge::tools_stories::role_from_name(identity);
    if let Err(e) = crate::bridge::dispatch_reprioritize_outcome(
        args,
        identity,
        role,
        ctx.routing_user_id.as_deref(),
        ctx.orchestrator.as_ref(),
        ctx.dispatch,
    )
    .await
    {
        send_json(
            tx,
            &ServerMessage::Error {
                reason: format!("reprioritize outcome failed: {e}"),
            },
        )
        .await?;
    }
    Ok(())
}
