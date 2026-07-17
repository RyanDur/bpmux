use super::acceptance_test_path::validate_acceptance_test_path_at_deliver;
use super::event_meta::meta_for;
use super::helpers::{ensure_story_in_canonical_store, require_nonempty_string};
use crate::sync::SyncOrchestrator;
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{Story, StoryId, Verdict};

pub(crate) async fn handle_deliver_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<SyncOrchestrator>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let transition = workflow::resolve_transition(workflow, "deliver", workflow::GateClass::None)
        .map_err(|e| e.as_json_string())?;
    workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_name().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let refreshed: Option<Option<Story>> = match refresh {
        Some(r) => {
            let story = ensure_story_in_canonical_store(r, select, id).await?;
            let _ = r.fetch_body(id).await;
            Some(Some(story))
        }
        None => None,
    };
    if let Some(story) = refreshed.as_ref().and_then(|s| s.as_ref()) {
        workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
    }
    if refresh.is_some() {
        validate_acceptance_test_path_at_deliver(id, select)?;
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
            .map(|s| s.is_concluded())
            .unwrap_or(false);
        let meta = meta_for(agent_name, actor_id);
        let held_step = refreshed
            .as_ref()
            .and_then(|s| s.as_ref())
            .and_then(|story| select.select(|state| board::held_stage(story, &state.board)))
            .unwrap_or_else(|| product_management::story::Stage::Dev.id());
        let board::Decision {
            coarse: new_state,
            placement,
        } = board::advance(
            &product_management::story::Verdict::Succeed {
                step: held_step.clone(),
            },
            workflow,
        )
        .ok_or_else(|| {
            format!(
                "#{id} deliver has no forward state from '{}'",
                held_step.as_str()
            )
        })?;
        let mut events = Vec::new();
        if was_closed {
            events.push(StoryEvent::new(
                meta.clone(),
                StoryId::new(id),
                StoryEventKind::StoryReopened,
            ));
        }
        events.push(StoryEvent::new(
            meta.clone(),
            StoryId::new(id),
            StoryEventKind::VerdictCast {
                verdict: Verdict::Succeed {
                    step: "in-progress".into(),
                },
                payload: Some(verified_observation.to_string()),
            },
        ));
        events.push(StoryEvent::new(
            meta,
            StoryId::new(id),
            StoryEventKind::StateTransitioned {
                new_state,
                placement,
            },
        ));
        r.mutate_batch(events)
            .map_err(|e| format!("#{id} deliver enqueue failed: {e}"))?;
        return Ok(format!("#{id} delivered: signals sent, labels updated"));
    }
    Err(
        "deliver_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
