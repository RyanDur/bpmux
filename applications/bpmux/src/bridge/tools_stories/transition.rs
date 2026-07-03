use super::acceptance_test_path::{
    validate_acceptance_test_path_at_deliver, validate_acceptance_test_path_at_pickup,
};
use super::helpers::{ensure_story_in_canonical_store, require_nonempty_string, role_from_name};
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::{PeerId, RefreshDriver, StoryEvent};
use crate::product_management::story::{BoardState, Gate, Stage, Story, StoryId};

pub(crate) async fn handle_start_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition =
        crate::workflow::resolve_transition(workflow, "pickup", crate::workflow::GateClass::None)
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
    let (was_closed, prior_state, reject_lead) = if let Some(r) = refresh {
        let prior = ensure_story_in_canonical_store(r, id).await?;
        let _ = r.fetch_body(id).await;
        let _ = r.fetch_gate_comments(id).await;
        let lead = r
            .story(id)
            .and_then(|s| s.gate_comments)
            .as_deref()
            .and_then(super::super::tools::rework_context::latest_reject)
            .and_then(super::super::tools::rework_context::reject_lead);
        (prior.is_closed, Some(prior.state), lead)
    } else {
        (false, None, None)
    };
    if let Some(state) = prior_state {
        crate::workflow::check_target_state_valid(transition, state.as_str())
            .map_err(|e| e.as_json_string())?;
    }
    let pickup_paraphrase = require_nonempty_string(
        args,
        "pickup_paraphrase",
        "pickup_paraphrase is required (2026-04-23 retro): restate the user-job as you understand it at pickup time. This broadcast is what separates 'dev is working' from 'dev is silent' to the rest of the team.",
    )?;
    let falsifier_reframing = require_nonempty_string(
        args,
        "falsifier_reframing",
        "falsifier_reframing is required (2026-04-23 retro): restate the observation-falsifier — either verbatim from filing or your sharpening. Forces engagement with the rejection criteria before code.",
    )?;
    if let Some(r) = refresh {
        validate_acceptance_test_path_at_pickup(args, id, r.as_ref())?;
    }
    let pickup_payload =
        format!("{pickup_paraphrase}\n\n---\n\n**falsifier_reframing**:\n\n{falsifier_reframing}");
    if let Some(r) = refresh {
        let meta = EventMeta {
            actor: crate::routing::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: PeerId::local(),
        };
        if was_closed {
            r.mutate_sync(StoryEvent::StoryReopened {
                meta: meta.clone(),
                id: StoryId::new(id),
            })
            .map_err(|e| format!("#{id} reopen enqueue failed: {e}"))?;
        }
        r.mutate_sync(StoryEvent::GateCommentAppended {
            meta: meta.clone(),
            id: StoryId::new(id),
            gate: Gate::Pickup,
            payload: Some(pickup_payload.clone()),
        })
        .map_err(|e| format!("#{id} gate enqueue failed: {e}"))?;
        r.mutate_sync(StoryEvent::StateTransitioned {
            meta,
            id: StoryId::new(id),
            new_state: BoardState::InProgress(Stage::Dev.id()),
        })
        .map_err(|e| format!("#{id} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{id} start push failed: {e}"))?;
        return Ok(match reject_lead {
            Some(lead) => format!("{lead}\n#{id} marked as in progress"),
            None => format!("#{id} marked as in progress"),
        });
    }
    Err(
        "start_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

pub(crate) async fn handle_deliver_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition =
        crate::workflow::resolve_transition(workflow, "deliver", crate::workflow::GateClass::None)
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
    let refreshed: Option<Option<Story>> = match refresh {
        Some(r) => {
            let story = ensure_story_in_canonical_store(r, id).await?;
            let _ = r.fetch_body(id).await;
            Some(Some(story))
        }
        None => None,
    };
    if let Some(story) = refreshed.as_ref().and_then(|s| s.as_ref()) {
        crate::workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
    }
    if let Some(r) = refresh {
        validate_acceptance_test_path_at_deliver(id, r.as_ref())?;
        if let Some(story) = refreshed.as_ref().and_then(|s| s.as_ref())
            && let Some(body) = story.body.as_deref()
        {
            crate::bridge::deliver_story_body_diff_gate::verify_deliver_drift(
                body,
                story,
                &|args| crate::bridge::tools::git_repo::run_git(args),
            )?;
        }
    }
    let verified_observation = require_nonempty_string(
        args,
        "verified_observation",
        "verified_observation is required (2026-04-22 retro): articulate what you DROVE end-to-end in the running app, not 'tests pass'. Describe the falsifier scenario you exercised + what you observed. Becomes the diff-point at desk-check.",
    )?;
    if let Some(r) = refresh {
        let was_closed = refreshed
            .as_ref()
            .and_then(|s| s.as_ref())
            .map(|s| s.is_closed)
            .unwrap_or(false);
        let meta = EventMeta {
            actor: crate::routing::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: PeerId::local(),
        };
        if was_closed {
            r.mutate_sync(StoryEvent::StoryReopened {
                meta: meta.clone(),
                id: StoryId::new(id),
            })
            .map_err(|e| format!("#{id} reopen enqueue failed: {e}"))?;
        }
        r.mutate_sync(StoryEvent::GateCommentAppended {
            meta: meta.clone(),
            id: StoryId::new(id),
            gate: Gate::Delivered,
            payload: Some(verified_observation.to_string()),
        })
        .map_err(|e| format!("#{id} gate enqueue failed: {e}"))?;
        r.mutate_sync(StoryEvent::StateTransitioned {
            meta,
            id: StoryId::new(id),
            new_state: BoardState::InProgress(Stage::DeskCheck.id()),
        })
        .map_err(|e| format!("#{id} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{id} drain after deliver failed: {e}"))?;
        return Ok(format!("#{id} delivered: signals sent, labels updated"));
    }
    Err(
        "deliver_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
