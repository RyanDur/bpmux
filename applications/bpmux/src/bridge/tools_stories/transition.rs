use super::acceptance_test_path::validate_acceptance_test_path_at_pickup;
use super::event_meta::meta_for;
use super::helpers::{ensure_story_in_canonical_store, require_nonempty_string};
use crate::sync::RefreshDriver;
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{StoryId, Verdict, Work};

pub(crate) async fn handle_start_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let transition = workflow::resolve_transition(workflow, "pickup", workflow::GateClass::None)
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
    let (was_closed, prior_state, reject_lead) = if let Some(r) = refresh {
        let prior = ensure_story_in_canonical_store(r, id).await?;
        let _ = r.fetch_body(id).await;
        if prior.verdicts.is_none() {
            let _ = r.fetch_verdicts(id).await;
        }
        let lead = r
            .story(id)
            .and_then(|s| s.verdicts)
            .as_deref()
            .and_then(super::super::tools::rework_context::latest_reject)
            .and_then(super::super::tools::rework_context::reject_lead);
        (prior.is_concluded(), Some(prior.state), lead)
    } else {
        (false, None, None)
    };
    if let Some(state) = prior_state {
        workflow::check_target_state_valid(transition, state.as_str())
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
        let meta = meta_for(agent_name, actor_id);
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
                    step: "ready".into(),
                },
                payload: Some(pickup_payload.clone()),
            },
        ));
        events.push(StoryEvent::new(
            meta,
            StoryId::new(id),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ));
        r.mutate_batch_sync(events)
            .map_err(|e| format!("#{id} start enqueue failed: {e}"))?;
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
