use tokio::sync::Mutex;

use super::super::tools::{WsSink, ws_send};
use super::helpers::{
    KEEP_NONE, ensure_story_in_canonical_store, reap_workflow_labels, require_purpose,
    role_from_name,
};
use super::signals::build_iteration_close_signal;

const REASON_CITE_MIN_CHARS: usize = 16;

pub(crate) async fn handle_close_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
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
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
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
        ensure_story_in_canonical_store(r, number).await?;
        let meta = crate::product_management::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::board::PeerId::local(),
        };
        reap_workflow_labels(r.as_ref(), number, &meta, KEEP_NONE)?;
        if disposition == "retire" || disposition == "wontfix" {
            r.mutate_sync(crate::product_management::board::BoardEvent::TagAdded {
                meta: meta.clone(),
                number,
                label: "retired".to_string(),
            })
            .map_err(|e| format!("#{number} retired-tag enqueue failed: {e}"))?;
            r.mutate_sync(
                crate::product_management::board::BoardEvent::StateTransitioned {
                    meta: meta.clone(),
                    number,
                    new_state: crate::product_management::board::domain::StoryState::Retired,
                },
            )
            .map_err(|e| format!("#{number} retired-state enqueue failed: {e}"))?;
        }
        r.mutate_sync(crate::product_management::board::BoardEvent::StoryClosed {
            meta,
            number,
            reason: Some(reason.to_string()),
        })
        .map_err(|e| format!("#{number} close enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} close push failed: {e}"))?;
        return Ok(format!("#{number} closed: {reason}"));
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
    let iteration_number = args.get("iteration_number").and_then(|v| v.as_u64());
    post_close_iteration(port, iteration_number).await?;
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

async fn post_close_iteration(port: u16, iteration_number: Option<u64>) -> Result<(), String> {
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::auth::read(&token_path)
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
    Ok(())
}
