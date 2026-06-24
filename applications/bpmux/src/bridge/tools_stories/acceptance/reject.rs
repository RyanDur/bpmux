use crate::bridge::tools_stories::helpers::{
    LABEL_DELIVERED, LABEL_IN_PROGRESS, LABEL_READY, ensure_story_in_canonical_store,
    reap_workflow_labels, role_from_name,
};

pub(crate) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            crate::workflow::GateError::GateMismatch {
                actor_role: actor_role.to_string(),
                attempted_transition: "reject".to_string(),
                class: crate::workflow::GateClass::None,
                missing_evidence: vec!["reason-cite".to_string()],
                verdict_state: None,
            }
            .as_json_string()
        })?
        .to_string();
    let r = refresh.ok_or(
        "reject_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    let story_opt = r.store().story(number);
    let class = story_opt
        .as_ref()
        .map(|s| {
            crate::workflow::gate_class_from_labels_and_story_type(&s.raw_labels, s.story_type)
        })
        .unwrap_or(crate::workflow::GateClass::TeamVerifiable);
    let transition = crate::workflow::resolve_transition(workflow, "reject", class)
        .map_err(|e| e.as_json_string())?;
    crate::workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_id().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    if let Some(story) = story_opt.as_ref() {
        crate::workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
    }
    let target_state = crate::product_management::story::StoryState::parse(&transition.to)
        .ok_or_else(|| {
            format!(
                "workflow.toml reject.to '{}' is not a known state",
                transition.to
            )
        })?;
    let actor = crate::transport::ws::short_name(agent_name).to_string();
    reject_via_canonical_store(r, number, actor, reason.clone(), target_state).await?;
    Ok(format!("#{number} rejected: {reason}"))
}

pub(crate) async fn reject_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    number: u64,
    actor: String,
    reason: String,
    target_state: crate::product_management::story::StoryState,
) -> Result<(), String> {
    let prior = ensure_story_in_canonical_store(refresh, number).await?;
    let was_accepted =
        prior.is_closed || prior.state == crate::product_management::story::StoryState::Accepted;
    let meta = crate::product_management::board::events::EventMeta {
        actor,
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    let keep_label = match target_state {
        crate::product_management::story::StoryState::Ready => LABEL_READY,
        crate::product_management::story::StoryState::InProgress => LABEL_IN_PROGRESS,
        crate::product_management::story::StoryState::Delivered => LABEL_DELIVERED,
        _ => LABEL_READY,
    };
    reap_workflow_labels(refresh.as_ref(), number, &meta, keep_label)?;
    if was_accepted {
        refresh
            .mutate_sync(
                crate::product_management::board::BoardEvent::StoryReopened {
                    meta: meta.clone(),
                    number,
                },
            )
            .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
    }
    refresh
        .mutate_sync(
            crate::product_management::board::BoardEvent::GateCommentAppended {
                meta: meta.clone(),
                number,
                gate: crate::product_management::story::Gate::Rejected,
                payload: Some(reason),
            },
        )
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
    refresh
        .mutate_sync(
            crate::product_management::board::BoardEvent::StateTransitioned {
                meta,
                number,
                new_state: target_state,
            },
        )
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
    refresh
        .drain_pending()
        .await
        .map_err(|e| format!("#{number} drain after reject failed: {e}"))?;
    Ok(())
}
