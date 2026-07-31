use std::sync::Arc;

use serde_json::Map;
use serde_json::Value;

use crate::tools_stories::event_meta::meta_for;
use product_management::story::StoryId;
use product_management::story::events::{StoryEvent, StoryEventKind};
use sync::SyncOrchestrator;

mod authorization;
mod emit;
mod scheduling;
mod target;

use emit::{emit_reopen_and_label, emit_transition};
use target::{
    TargetState, audit_comment_body, current_state_display, parse_position, require_nonempty,
    validate_position,
};

pub async fn handle_move_story(
    args: &Map<String, Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<SyncOrchestrator>>,
    dispatch: &store::Dispatch,
    select: &store::Select,
) -> Result<String, String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let target_str = args
        .get("target_state")
        .and_then(|v| v.as_str())
        .ok_or("missing 'target_state' argument")?;
    let target = TargetState::parse(target_str)?;
    let position = parse_position(args)?;
    let rationale = require_nonempty(
        args,
        "rationale",
        "rationale is required (#1188 falsifier-b — every move must surface an audit-trail reason)",
    )?;

    authorization::enforce_move_authorization(role)?;

    validate_position(position, &target, id)?;

    let r = refresh.ok_or("move_story requires refresh — tracker fallback not supported")?;
    let prior = Some(crate::tools_stories::ensure_story_in_canonical_store(r, select, id).await?);
    let from_fine = prior
        .as_ref()
        .and_then(|s| select.select(|state| board::fine_stage(s, &state.board)));
    let from_display = current_state_display(
        prior.as_ref(),
        from_fine,
        &select.select(|state| product_management::iteration::all(&state.iterations)),
    )
    .to_string();
    let was_closed = prior.as_ref().map(|s| s.is_concluded()).unwrap_or(false);

    let scheduling_target = scheduling::resolve_scheduling_target(select, id, &target)?;

    let meta = meta_for(agent_name, actor_id);
    let actor_short = meta.actor.clone();
    let story_id = StoryId::new(id);

    emit_reopen_and_label(r, &meta, story_id, &target, prior.as_ref(), was_closed)?;
    emit_transition(
        r, &meta, story_id, &target, target_str, was_closed, &rationale,
    )?;

    scheduling::apply_scheduling(
        r,
        select,
        dispatch,
        id,
        &target,
        position,
        scheduling_target,
    )
    .await?;

    let body = audit_comment_body(&actor_short, &from_display, &target, position, &rationale);
    r.mutate(StoryEvent::new(
        meta,
        story_id,
        StoryEventKind::FreeCommentAppended { body },
    ))
    .map_err(|e| format!("#{id} audit-comment enqueue failed: {e}"))?;

    let pos_msg = match position {
        Some(p) => format!(" at position {p}"),
        None => String::new(),
    };
    Ok(format!(
        "#{id} moved {from_display} → {to}{pos_msg}",
        to = target.as_display()
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "move_story_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "move_story_ready_order_burst_tests.rs"]
mod move_story_ready_order_burst_tests;

#[cfg(test)]
#[path = "move_story_scheduling_tests.rs"]
mod move_story_scheduling_tests;
