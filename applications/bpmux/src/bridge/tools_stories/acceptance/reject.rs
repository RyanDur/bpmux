use tokio::sync::Mutex;

use crate::bridge::tools::{WsSink, ws_send};
use crate::bridge::tools_stories::helpers::{
    LABEL_DELIVERED, LABEL_IN_PROGRESS, LABEL_READY, ensure_story_in_canonical_store,
    reap_workflow_labels, role_from_name,
};
use crate::bridge::tools_stories::signals::{broadcast_board_invalidate, build_reject_signal};

pub(crate) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
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
    let target_state =
        crate::board::domain::StoryState::parse(&transition.to).ok_or_else(|| {
            format!(
                "workflow.toml reject.to '{}' is not a known state",
                transition.to
            )
        })?;
    let signal = serde_json::to_string(&build_reject_signal(number, &reason))
        .map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
    }
    let actor = crate::transport::ws::short_name(agent_name).to_string();
    reject_via_canonical_store(r, number, actor, reason.clone(), target_state).await?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} rejected: {reason}"))
}

pub(crate) async fn reject_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
    actor: String,
    reason: String,
    target_state: crate::board::domain::StoryState,
) -> Result<(), String> {
    let prior = ensure_story_in_canonical_store(refresh, number).await?;
    let was_accepted = prior.is_closed || prior.state == crate::board::domain::StoryState::Accepted;
    let meta = crate::board::events::EventMeta {
        actor,
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    let keep_label = match target_state {
        crate::board::domain::StoryState::Ready => LABEL_READY,
        crate::board::domain::StoryState::InProgress => LABEL_IN_PROGRESS,
        crate::board::domain::StoryState::Delivered => LABEL_DELIVERED,
        _ => LABEL_READY,
    };
    reap_workflow_labels(refresh.as_ref(), number, &meta, keep_label)?;
    if was_accepted {
        refresh
            .mutate_sync(crate::board::BoardEvent::StoryReopened {
                meta: meta.clone(),
                number,
            })
            .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
    }
    refresh
        .mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::Rejected,
            payload: Some(reason),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
    refresh
        .mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: target_state,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
    refresh
        .drain_pending()
        .await
        .map_err(|e| format!("#{number} drain after reject failed: {e}"))?;
    Ok(())
}
