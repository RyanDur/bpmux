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
    let was_accepted = prior.as_ref().map(|s| s.is_concluded()).unwrap_or(false);
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
    let crate::board::Decision {
        coarse: new_state,
        placement,
    } = crate::board::advance(
        &crate::product_management::story::Verdict::Fail {
            step: transition.from.as_str().into(),
        },
        workflow,
    )
    .ok_or_else(|| {
        format!(
            "#{id} fail-desk-check has no target from '{}'",
            transition.from
        )
    })?;
    if let Some(r) = refresh {
        let meta = crate::product_management::story::events::EventMeta {
            actor: crate::routing::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::story::peer::PeerId::local(),
        };
        let mut events = Vec::new();
        if was_accepted {
            events.push(
                crate::product_management::story::events::StoryEvent::StoryReopened {
                    meta: meta.clone(),
                    id: StoryId::new(id),
                },
            );
        }
        events.push(
            crate::product_management::story::events::StoryEvent::VerdictCast {
                meta: meta.clone(),
                id: StoryId::new(id),
                verdict: crate::product_management::story::Verdict::Fail {
                    step: "desk-check".into(),
                },
                payload: Some(reason.to_string()),
            },
        );
        events.push(
            crate::product_management::story::events::StoryEvent::StateTransitioned {
                meta,
                id: StoryId::new(id),
                placement,
                new_state,
            },
        );
        r.mutate_batch_sync(events)
            .map_err(|e| format!("#{id} fail-desk-check enqueue failed: {e}"))?;
        return Ok(format!(
            "#{id} failed desk-check: signal sent, label applied"
        ));
    }
    Err(
        "fail_desk_check requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
