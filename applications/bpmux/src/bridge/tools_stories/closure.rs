use tokio::sync::Mutex;

use super::super::tools::{WsSink, ws_send};
use super::helpers::{ensure_story_in_canonical_store, require_purpose, role_from_name};
use super::signals::build_iteration_close_signal;
use crate::product_management::story::StoryId;

const REASON_CITE_MIN_CHARS: usize = 16;

pub(crate) async fn handle_close_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition =
        crate::workflow::resolve_transition(workflow, "close", crate::workflow::GateClass::None)
            .map_err(|e| e.as_json_string())?;
    crate::workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_id().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    let disposition = args
        .get("disposition")
        .and_then(|v| v.as_str())
        .unwrap_or("retire");
    let trimmed_reason = reason.trim();
    if trimmed_reason.len() < REASON_CITE_MIN_CHARS {
        let err = serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "close",
            "missing_evidence": ["reason-cite"],
            "min_chars": REASON_CITE_MIN_CHARS,
            "observed_chars": trimmed_reason.len(),
        });
        return Err(err.to_string());
    }
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, id).await?;
        let meta = crate::product_management::story::events::EventMeta {
            actor: crate::routing::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::story::peer::PeerId::local(),
        };
        if disposition == "retire" || disposition == "wontfix" {
            r.mutate_sync(
                crate::product_management::story::events::StoryEvent::TagAdded {
                    meta: meta.clone(),
                    id: StoryId::new(id),
                    label: "retired".to_string(),
                },
            )
            .map_err(|e| format!("#{id} retired-tag enqueue failed: {e}"))?;
            r.mutate_sync(
                crate::product_management::story::events::StoryEvent::StateTransitioned {
                    meta: meta.clone(),
                    id: StoryId::new(id),
                    new_state: crate::product_management::story::BoardState::Retired,
                },
            )
            .map_err(|e| format!("#{id} retired-state enqueue failed: {e}"))?;
        }
        r.mutate_sync(
            crate::product_management::story::events::StoryEvent::StoryClosed {
                meta,
                id: StoryId::new(id),
                reason: Some(reason.to_string()),
            },
        )
        .map_err(|e| format!("#{id} close enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{id} close push failed: {e}"))?;
        return Ok(format!("#{id} closed: {reason}"));
    }
    Err(
        "close_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

pub(crate) async fn handle_close_iteration(
    agent_name: &str,
    args: &serde_json::Map<String, serde_json::Value>,
    ws_tx: &Mutex<Option<WsSink>>,
    port: u16,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "close iteration")?;
    let iteration_id = args.get("iteration_id").and_then(|v| v.as_u64());
    post_close_iteration(port, iteration_id).await?;
    let signal = build_iteration_close_signal();
    let json = serde_json::to_string(&signal).map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, json).await {
        crate::diag::log_diag(&format!("bridge: iteration-close signal send failed: {e}"));
    }
    Ok(
        "iteration closed (terminal; promote-on-close retired #1556) — agents prompted to flush unsaved findings."
            .to_string(),
    )
}

async fn post_close_iteration(port: u16, iteration_id: Option<u64>) -> Result<(), String> {
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::auth::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, "/api/close_iteration");
    let body = serde_json::json!({ "iteration_id": iteration_id });
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
    Ok(())
}
