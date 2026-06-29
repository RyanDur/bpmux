use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::SplitSink;

use super::client::ClientCtx;
use super::send_json;
use crate::transport::protocol::ServerMessage;

pub(super) async fn handle_set_outcome_horizon(
    ctx: &ClientCtx<'_>,
    tx: &mut SplitSink<WebSocket, WsMessage>,
    identity: &str,
    outcome_number: u64,
    horizon: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "outcome_number".to_string(),
        serde_json::json!(outcome_number),
    );
    args.insert("to".to_string(), serde_json::json!(horizon));
    if let Err(e) =
        crate::bridge::dispatch_reprioritize_outcome(args, identity, ctx.orchestrator.as_ref())
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
    outcome_number: u64,
    iteration_number: u64,
    position: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "outcome_number".to_string(),
        serde_json::json!(outcome_number),
    );
    args.insert(
        "iteration_number".to_string(),
        serde_json::json!(iteration_number),
    );
    args.insert("position".to_string(), serde_json::json!(position));
    if let Err(e) =
        crate::bridge::dispatch_reorder_outcome_iteration(args, identity, ctx.orchestrator.as_ref())
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
    iteration_number: u64,
    position: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "iteration_number".to_string(),
        serde_json::json!(iteration_number),
    );
    args.insert("position".to_string(), serde_json::json!(position));
    if let Err(e) =
        crate::bridge::dispatch_reorder_iteration(args, identity, ctx.orchestrator.as_ref()).await
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
    iteration_number: u64,
    story_number: u64,
    position: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "iteration_number".to_string(),
        serde_json::json!(iteration_number),
    );
    args.insert("story_number".to_string(), serde_json::json!(story_number));
    args.insert("position".to_string(), serde_json::json!(position));
    if let Err(e) =
        crate::bridge::dispatch_add_story_to_iteration(args, identity, ctx.orchestrator.as_ref())
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
    outcome_number: u64,
    position: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = serde_json::Map::new();
    args.insert(
        "outcome_number".to_string(),
        serde_json::json!(outcome_number),
    );
    args.insert("position".to_string(), serde_json::json!(position));
    if let Err(e) =
        crate::bridge::dispatch_reprioritize_outcome(args, identity, ctx.orchestrator.as_ref())
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
