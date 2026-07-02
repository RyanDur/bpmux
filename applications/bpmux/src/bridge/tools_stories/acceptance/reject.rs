use crate::bridge::tools_stories::helpers::{ensure_story_in_canonical_store, role_from_name};
use crate::product_management::story::StoryId;

pub(crate) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
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
    let story_opt = r.story(id);
    let class = story_opt
        .as_ref()
        .map(|s| {
            crate::workflow::gate_class_from_acceptor_and_story_type(s.acceptor(), s.story_type())
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
    let target_state = crate::product_management::story::BoardState::parse(&transition.to)
        .ok_or_else(|| {
            format!(
                "workflow.toml reject.to '{}' is not a known state",
                transition.to
            )
        })?;
    let actor = crate::transport::ws::short_name(agent_name).to_string();
    reject_via_canonical_store(r, id, actor, reason.clone(), target_state).await?;
    Ok(format!("#{id} rejected: {reason}"))
}

pub(crate) async fn reject_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    id: u64,
    actor: String,
    reason: String,
    target_state: crate::product_management::story::BoardState,
) -> Result<(), String> {
    let prior = ensure_story_in_canonical_store(refresh, id).await?;
    let was_accepted =
        prior.is_closed || prior.state == crate::product_management::story::BoardState::Accepted;
    let meta = crate::product_management::board::events::EventMeta {
        actor,
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    if was_accepted {
        refresh
            .mutate_sync(
                crate::product_management::board::StoryEvent::StoryReopened {
                    meta: meta.clone(),
                    id: StoryId::new(id),
                },
            )
            .map_err(|e| format!("#{id} reopen enqueue failed: {e}"))?;
    }
    refresh
        .mutate_sync(
            crate::product_management::board::StoryEvent::GateCommentAppended {
                meta: meta.clone(),
                id: StoryId::new(id),
                gate: crate::product_management::story::Gate::Rejected,
                payload: Some(reason),
            },
        )
        .map_err(|e| format!("#{id} gate enqueue failed: {e}"))?;
    refresh
        .mutate_sync(
            crate::product_management::board::StoryEvent::StateTransitioned {
                meta,
                id: StoryId::new(id),
                new_state: target_state,
            },
        )
        .map_err(|e| format!("#{id} state enqueue failed: {e}"))?;
    refresh
        .drain_pending()
        .await
        .map_err(|e| format!("#{id} drain after reject failed: {e}"))?;
    Ok(())
}
