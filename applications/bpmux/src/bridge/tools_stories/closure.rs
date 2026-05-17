//! Closure handlers: close_story (single-story retire) and
//! close_iteration (PM-gated iteration externalize-prompt + server-
//! side promote/close ceremony).

use tokio::sync::Mutex;

use super::super::tools::{WsSink, ws_send};
use super::helpers::{
    KEEP_NONE, ensure_story_in_canonical_store, reap_workflow_labels, require_role,
};
use super::signals::{broadcast_board_invalidate, build_iteration_close_signal};

/// Close a story with a reason — PM-only. Routes through the
/// RefreshDriver per #451 to append the reason as a comment, then
/// fire StoryClosed; accept_story uses the same close path.
/// Generalizes the invocation for superseded / duplicate / won't-fix
/// closures.
pub(crate) async fn handle_close_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "close")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    if reason.trim().is_empty() {
        return Err("'reason' must not be empty — audit trail for closure".to_string());
    }
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, number).await?;
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        reap_workflow_labels(r.as_ref(), number, &meta, KEEP_NONE)?;
        r.mutate_sync(crate::board::BoardEvent::StoryClosed {
            meta,
            number,
            reason: Some(reason.to_string()),
        })
        .map_err(|e| format!("#{number} close enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} close push failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} closed: {reason}"));
    }
    Err(
        "close_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

/// Fire the iteration-close externalize-prompt (#282). PM-gated.
///
/// Fail-soft: the MCP tool returns after the broadcast ws_send
/// completes. Iteration closes regardless of whether any agent
/// responds to the prompt — the tool is a reminder, not a barrier.
pub(crate) async fn handle_close_iteration(
    agent_name: &str,
    args: &serde_json::Map<String, serde_json::Value>,
    ws_tx: &Mutex<Option<WsSink>>,
    port: u16,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "close iteration")?;
    let iteration_number = args.get("iteration_number").and_then(|v| v.as_u64());
    let outcome = post_close_iteration(port, iteration_number).await?;
    let signal = build_iteration_close_signal();
    let json = serde_json::to_string(&signal).map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, json).await {
        crate::session::diag::log_diag(&format!("bridge: iteration-close signal send failed: {e}"));
    }
    Ok(format!(
        "iteration-close signal broadcast — agents prompted to flush unsaved findings. \
         promoted: {} stories; deleted marker: {}",
        outcome.promoted.len(),
        outcome
            .deleted_marker_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none (no markers in icebox)".to_string()),
    ))
}

#[derive(Debug, Default, serde::Deserialize)]
struct CloseIterationOutcomeJson {
    #[serde(default)]
    promoted: Vec<u64>,
    #[serde(default)]
    deleted_marker_id: Option<u64>,
}

async fn post_close_iteration(
    port: u16,
    iteration_number: Option<u64>,
) -> Result<CloseIterationOutcomeJson, String> {
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, "/api/close_iteration");
    let body = serde_json::json!({ "iteration_number": iteration_number });
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("close_iteration: bpmux-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("close_iteration: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("close_iteration: {status} — {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("close_iteration: parse outcome: {e}"))
}
