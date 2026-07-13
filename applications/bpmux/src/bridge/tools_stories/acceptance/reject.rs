use crate::bridge::tools_stories::event_meta::meta_for;
use crate::bridge::tools_stories::helpers::ensure_story_in_canonical_store;
use product_management::story::StoryId;

pub(crate) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            workflow::GateError::GateMismatch {
                actor_role: actor_role.to_string(),
                attempted_transition: "reject".to_string(),
                class: workflow::GateClass::None,
                missing_evidence: vec!["reason-cite".to_string()],
                verdict_state: None,
            }
            .as_json_string()
        })?
        .to_string();
    let r = refresh.ok_or(
        "reject_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    let story_opt = r.story(id);
    let class = story_opt
        .as_ref()
        .map(|s| {
            product_management::story::gate_class_from_acceptor_and_story_type(
                s.acceptor(),
                s.story_type(),
            )
        })
        .unwrap_or(workflow::GateClass::TeamVerifiable);
    let transition =
        workflow::resolve_transition(workflow, "reject", class).map_err(|e| e.as_json_string())?;
    workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_name().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    if let Some(story) = story_opt.as_ref() {
        workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
    }
    reject_via_canonical_store(r, id, agent_name, actor_id, reason.clone()).await?;
    Ok(format!("#{id} rejected: {reason}"))
}

pub(crate) async fn reject_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    id: u64,
    agent_name: &str,
    actor_id: Option<&str>,
    reason: String,
) -> Result<(), String> {
    let prior = ensure_story_in_canonical_store(refresh, id).await?;
    let workflow = workflow::current_workflow();
    let transition = workflow
        .transition_by_id("reject")
        .ok_or("workflow has no 'reject' transition")?;
    let crate::board::Decision {
        coarse: new_state,
        placement,
    } = crate::board::advance(
        &product_management::story::Verdict::Fail {
            step: transition.from.as_str().into(),
        },
        workflow,
    )
    .ok_or_else(|| format!("#{id} reject has no target from '{}'", transition.from))?;
    let was_accepted = prior.is_concluded();
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
                step: "acceptance".into(),
            },
            payload: Some(reason),
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
    refresh
        .mutate_batch_sync(events)
        .map_err(|e| format!("#{id} reject enqueue failed: {e}"))?;
    Ok(())
}
