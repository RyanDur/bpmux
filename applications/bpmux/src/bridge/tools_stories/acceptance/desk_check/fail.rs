use crate::bridge::tools_stories::helpers::{ensure_story_in_canonical_store, role_from_name};
use crate::product_management::story::StoryId;

const FALSIFIER_CITE_MIN_CHARS: usize = 16;

pub(crate) async fn handle_fail_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition = crate::workflow::resolve_transition(
        workflow,
        "fail-desk-check",
        crate::workflow::GateClass::None,
    )
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
    let prior = if let Some(r) = refresh {
        Some(ensure_story_in_canonical_store(r, id).await?)
    } else {
        None
    };
    if let Some(story) = prior.as_ref() {
        let current_state = match refresh {
            Some(r) => crate::board::state_label(&story.state, r.story_fine_stage(story)),
            None => story.state.as_str(),
        };
        crate::workflow::check_target_state_valid(transition, current_state)
            .map_err(|e| e.as_json_string())?;
    }
    let was_accepted = prior
        .as_ref()
        .map(|s| s.is_closed || s.state == crate::product_management::story::BoardState::Accepted)
        .unwrap_or(false);
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    let trimmed_reason = reason.trim();
    if trimmed_reason.len() < FALSIFIER_CITE_MIN_CHARS {
        let err = serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "fail-desk-check",
            "missing_evidence": ["falsifier-cite"],
            "min_chars": FALSIFIER_CITE_MIN_CHARS,
            "observed_chars": trimmed_reason.len(),
        });
        return Err(err.to_string());
    }
    let target_state = crate::product_management::story::BoardState::parse(transition.to.as_str())
        .ok_or_else(|| {
            format!(
                "workflow.toml fail-desk-check transition.to='{}' is not a recognized BoardState",
                transition.to
            )
        })?;
    if let Some(r) = refresh {
        let meta = crate::product_management::story::events::EventMeta {
            actor: crate::routing::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::story::peer::PeerId::local(),
        };
        if was_accepted {
            r.mutate_sync(
                crate::product_management::story::events::StoryEvent::StoryReopened {
                    meta: meta.clone(),
                    id: StoryId::new(id),
                },
            )
            .map_err(|e| format!("#{id} reopen enqueue failed: {e}"))?;
        }
        r.mutate_sync(
            crate::product_management::story::events::StoryEvent::GateCommentAppended {
                meta: meta.clone(),
                id: StoryId::new(id),
                gate: crate::product_management::story::Gate::FailDeskCheck,
                payload: Some(reason.to_string()),
            },
        )
        .map_err(|e| format!("#{id} gate enqueue failed: {e}"))?;
        r.mutate_sync(
            crate::product_management::story::events::StoryEvent::StateTransitioned {
                meta,
                id: StoryId::new(id),
                placement: None,
                new_state: target_state,
            },
        )
        .map_err(|e| format!("#{id} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{id} drain after fail_desk_check failed: {e}"))?;
        return Ok(format!(
            "#{id} failed desk-check: signal sent, label applied"
        ));
    }
    Err(
        "fail_desk_check requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
