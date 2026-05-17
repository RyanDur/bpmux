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
///
/// #1377: reasons matching retire-class keywords (superseded /
/// duplicate / won't-fix) add the `retired` label before
/// `StoryClosed` fires, so the canonical state-axis lands as
/// `StoryState::Retired` (distinct from ship-chain accept) and the
/// renderer routes the story to the retired column instead of
/// polluting accepted.
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
        if is_retire_reason(reason) {
            r.mutate_sync(crate::board::BoardEvent::TagAdded {
                meta: meta.clone(),
                number,
                label: "retired".to_string(),
            })
            .map_err(|e| format!("#{number} retired-tag enqueue failed: {e}"))?;
            r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
                meta: meta.clone(),
                number,
                new_state: crate::board::domain::StoryState::Retired,
            })
            .map_err(|e| format!("#{number} retired-state enqueue failed: {e}"))?;
        }
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

/// Classify a close_story reason as retire-class (superseded /
/// duplicate / won't-fix) vs accept-class (everything else, including
/// no-reason). Matches by first-token / contains so PM-prose like
/// "superseded — fold-into-touch pattern" classifies as retire while
/// "shipped clean" stays accept-class.
pub(crate) fn is_retire_reason(reason: &str) -> bool {
    let lower = reason.to_ascii_lowercase();
    lower.starts_with("superseded")
        || lower.starts_with("supersede")
        || lower.starts_with("duplicate")
        || lower.starts_with("won't-fix")
        || lower.starts_with("won't fix")
        || lower.starts_with("wont-fix")
        || lower.starts_with("wontfix")
        || lower.starts_with("retired")
}

#[cfg(test)]
mod tests {
    use super::is_retire_reason;

    #[test]
    fn classifies_canonical_retire_reasons() {
        assert!(is_retire_reason("superseded"));
        assert!(is_retire_reason("duplicate"));
        assert!(is_retire_reason("won't-fix"));
        assert!(is_retire_reason("wont-fix"));
        assert!(is_retire_reason("retired"));
    }

    #[test]
    fn classifies_pm_prose_starting_with_retire_keyword() {
        assert!(is_retire_reason("superseded — fold-into-touch pattern"));
        assert!(is_retire_reason("duplicate of #1283"));
        assert!(is_retire_reason("Won't fix — out of scope"));
    }

    #[test]
    fn rejects_ship_chain_reasons() {
        assert!(!is_retire_reason("shipped clean"));
        assert!(!is_retire_reason("accepted via desk-check"));
        assert!(!is_retire_reason("done"));
    }

    #[test]
    fn rejects_unrelated_prose() {
        assert!(!is_retire_reason("the feature looks great"));
        assert!(!is_retire_reason(""));
    }
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
