use std::sync::Arc;

use serde_json::Map;
use serde_json::Value;

use crate::bridge::tools_stories::event_meta::meta_for;
use crate::sync::{RefreshDriver, SyncOrchestrator};
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{StoryId, Work};

mod authorization;
mod scheduling;
mod target;

use target::{
    TargetState, audit_comment_body, current_state_display, parse_position, require_nonempty,
};

pub(in crate::bridge) async fn handle_move_story(
    args: &Map<String, Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<SyncOrchestrator>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
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

    if position.is_some() {
        match target {
            TargetState::Ready => {}
            TargetState::Icebox => {
                return Err(format!(
                    "#{id} cannot move to the icebox at a position: the icebox is unordered — move to icebox without a position"
                ));
            }
            _ => {
                return Err(format!(
                    "'position' only meaningful for the ready column; target_state={} ignores ordering",
                    target.as_display()
                ));
            }
        }
    }

    let r = refresh
        .ok_or("move_story requires refresh (RefreshDriver) — tracker fallback not supported")?;
    let prior =
        Some(crate::bridge::tools_stories::ensure_story_in_canonical_store(r, select, id).await?);
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

    if was_closed && !target.closes_issue() {
        r.mutate_sync(StoryEvent::new(
            meta.clone(),
            story_id,
            StoryEventKind::StoryReopened,
        ))
        .map_err(|e| format!("#{id} reopen enqueue failed: {e}"))?;
    }

    if let Some(label) = target.as_label() {
        let target_state = target.workflow_state().unwrap_or(Work::Retired);
        let already_labeled = prior
            .as_ref()
            .map(|s| s.state() == target_state)
            .unwrap_or(false);
        if !already_labeled {
            r.mutate_sync(StoryEvent::new(
                meta.clone(),
                story_id,
                StoryEventKind::TagAdded {
                    label: label.to_string(),
                },
            ))
            .map_err(|e| format!("#{id} tag-add enqueue failed: {e}"))?;
        }
    }

    match &target {
        TargetState::Icebox | TargetState::Ready | TargetState::Workflow(_) => {
            let state = target
                .workflow_state()
                .ok_or_else(|| format!("#{id} move target has no workflow state"))?;
            r.mutate_sync(StoryEvent::new(
                meta.clone(),
                story_id,
                StoryEventKind::StateTransitioned {
                    new_state: state.clone(),
                    placement: board::placement_from_target(
                        &target_str.to_ascii_lowercase().replace([' ', '_'], "-"),
                        &workflow::board_middle_stage_ids(workflow::current_workflow()),
                    ),
                },
            ))
            .map_err(|e| format!("#{id} state enqueue failed: {e}"))?;
            if matches!(state, Work::Accepted) && !was_closed {
                r.mutate_sync(StoryEvent::new(
                    meta.clone(),
                    story_id,
                    StoryEventKind::StoryClosed {
                        reason: Some(rationale.clone()),
                    },
                ))
                .map_err(|e| format!("#{id} accept-close enqueue failed: {e}"))?;
            }
        }
        TargetState::Retired => {
            r.mutate_sync(StoryEvent::new(
                meta.clone(),
                story_id,
                StoryEventKind::StateTransitioned {
                    new_state: Work::Retired,
                    placement: None,
                },
            ))
            .map_err(|e| format!("#{id} retire-state enqueue failed: {e}"))?;
            r.mutate_sync(StoryEvent::new(
                meta.clone(),
                story_id,
                StoryEventKind::StoryClosed {
                    reason: Some(format!("retired: {rationale}")),
                },
            ))
            .map_err(|e| format!("#{id} retire enqueue failed: {e}"))?;
        }
    }

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
    r.mutate_sync(StoryEvent::new(
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
