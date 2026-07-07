use super::helpers::{ensure_story_in_canonical_store, role_from_name};
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
    let terminal_state = match disposition {
        "retire" | "wontfix" => crate::product_management::story::BoardState::Retired,
        "accept" => crate::product_management::story::BoardState::Accepted,
        other => {
            return Err(format!(
                "unknown disposition '{other}' — expected one of: retire, wontfix, accept"
            ));
        }
    };
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
        if matches!(
            terminal_state,
            crate::product_management::story::BoardState::Retired
        ) {
            r.mutate_sync(
                crate::product_management::story::events::StoryEvent::TagAdded {
                    meta: meta.clone(),
                    id: StoryId::new(id),
                    label: "retired".to_string(),
                },
            )
            .map_err(|e| format!("#{id} retired-tag enqueue failed: {e}"))?;
        }
        r.mutate_sync(
            crate::product_management::story::events::StoryEvent::StateTransitioned {
                meta: meta.clone(),
                id: StoryId::new(id),
                new_state: terminal_state,
                placement: None,
            },
        )
        .map_err(|e| format!("#{id} close-state enqueue failed: {e}"))?;
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
