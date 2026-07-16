use crate::bridge::tools_stories::event_meta::meta_for;
use crate::bridge::tools_stories::helpers::ensure_story_in_canonical_store;
use product_management::story::StoryId;

const FALSIFIER_CITE_MIN_CHARS: usize = 16;

pub(crate) async fn handle_fail_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
) -> Result<String, String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let transition =
        workflow::resolve_transition(workflow, "fail-desk-check", workflow::GateClass::None)
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
    let prior = if let Some(r) = refresh {
        Some(ensure_story_in_canonical_store(r, select, id).await?)
    } else {
        None
    };
    if let Some(story) = prior.as_ref() {
        let current_state = match refresh {
            Some(_) => board::state_label(
                &product_management::scheduling::projected_state(
                    story,
                    &select.select(|state| product_management::iteration::all(&state.iterations)),
                ),
                select.select(|state| board::fine_stage(story, &state.board)),
            ),
            None => story.state.as_str(),
        };
        workflow::check_target_state_valid(transition, current_state)
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
    let board::Decision {
        coarse: new_state,
        placement,
    } = board::advance(
        &product_management::story::Verdict::Fail {
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
        let meta = meta_for(agent_name, actor_id);
        let mut events = Vec::new();
        if was_accepted {
            events.push(product_management::story::events::StoryEvent::new(
                meta.clone(),
                StoryId::new(id),
                product_management::story::events::StoryEventKind::StoryReopened,
            ));
        }
        events.push(product_management::story::events::StoryEvent::new(
            meta.clone(),
            StoryId::new(id),
            product_management::story::events::StoryEventKind::VerdictCast {
                verdict: product_management::story::Verdict::Fail {
                    step: "desk-check".into(),
                },
                payload: Some(reason.to_string()),
            },
        ));
        events.push(product_management::story::events::StoryEvent::new(
            meta,
            StoryId::new(id),
            product_management::story::events::StoryEventKind::StateTransitioned {
                placement,
                new_state,
            },
        ));
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
