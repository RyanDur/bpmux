use super::acceptance_test_path::validate_acceptance_test_path_at_deliver;
use super::helpers::{ensure_story_in_canonical_store, require_nonempty_string, role_from_name};
use crate::product_management::story::events::{EventMeta, StoryEvent};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{Story, StoryId, Verdict};
use crate::sync::RefreshDriver;

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
        let crate::board::Decision {
            coarse: new_state,
            placement,
        } = crate::board::advance(
            &crate::product_management::story::Verdict::Succeed {
                step: transition.from.as_str().into(),
            },
            workflow,
        )
        .ok_or_else(|| {
            format!(
                "#{id} deliver has no forward state from '{}'",
                transition.from
            )
        })?;
        let mut events = Vec::new();
        if was_closed {
            events.push(StoryEvent::StoryReopened {
                meta: meta.clone(),
                id: StoryId::new(id),
            });
        }
        events.push(StoryEvent::VerdictCast {
            meta: meta.clone(),
            id: StoryId::new(id),
            verdict: Verdict::Succeed {
                step: "in-progress".into(),
            },
            payload: Some(verified_observation.to_string()),
        });
        events.push(StoryEvent::StateTransitioned {
            meta,
            id: StoryId::new(id),
            new_state,
            placement,
        });
        r.mutate_batch_sync(events)
            .map_err(|e| format!("#{id} deliver enqueue failed: {e}"))?;
        return Ok(format!("#{id} delivered: signals sent, labels updated"));
    }
    Err(
        "deliver_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
