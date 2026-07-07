use std::sync::Arc;

use serde_json::Map;
use serde_json::Value;

use crate::product_management::iteration::label::current_iteration_id;
use crate::product_management::story::events::{EventMeta, StoryEvent};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{BoardState, StoryId};
use crate::sync::RefreshDriver;

mod authorization;
mod target;

use target::{
    TargetState, audit_comment_body, current_state_display, parse_position, require_nonempty,
};

pub(in crate::bridge) async fn handle_move_story(
    args: &Map<String, Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
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

    authorization::enforce_move_authorization(agent_name)?;

    if position.is_some() {
        let allows_position = matches!(
            target,
            TargetState::Workflow(BoardState::Ready) | TargetState::Workflow(BoardState::Icebox)
        );
        if !allows_position {
            return Err(format!(
                "'position' only meaningful for ready/icebox columns; target_state={} ignores ordering",
                target.as_display()
            ));
        }
    }

    let r = refresh
        .ok_or("move_story requires refresh (RefreshDriver) — tracker fallback not supported")?;
    let prior = Some(crate::bridge::tools_stories::ensure_story_in_canonical_store(r, id).await?);
    let from_fine = prior.as_ref().and_then(|s| r.story_fine_stage(s));
    let from_display = current_state_display(prior.as_ref(), from_fine).to_string();
    let was_closed = prior.as_ref().map(|s| s.is_concluded()).unwrap_or(false);

    let actor_short = crate::routing::short_name(agent_name).to_string();
    let meta = EventMeta {
        actor: actor_short.clone(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    };
    let story_id = StoryId::new(id);

    if was_closed && !target.closes_issue() {
        r.mutate_sync(StoryEvent::StoryReopened {
            meta: meta.clone(),
            id: story_id,
        })
        .map_err(|e| format!("#{id} reopen enqueue failed: {e}"))?;
    }

    if let Some(label) = target.as_label() {
        let target_state = match &target {
            TargetState::Workflow(state) => state.clone(),
            TargetState::Retired => BoardState::Retired,
        };
        let already_labeled = prior
            .as_ref()
            .map(|s| s.state() == target_state)
            .unwrap_or(false);
        if !already_labeled {
            r.mutate_sync(StoryEvent::TagAdded {
                meta: meta.clone(),
                id: story_id,
                label: label.to_string(),
            })
            .map_err(|e| format!("#{id} tag-add enqueue failed: {e}"))?;
        }
    }

    match &target {
        TargetState::Workflow(state) => {
            r.mutate_sync(StoryEvent::StateTransitioned {
                meta: meta.clone(),
                id: story_id,
                new_state: state.clone(),
                placement: crate::board::placement_from_target(
                    &target_str.to_ascii_lowercase().replace([' ', '_'], "-"),
                    &crate::workflow::board_middle_stage_ids(crate::workflow::current_workflow()),
                ),
            })
            .map_err(|e| format!("#{id} state enqueue failed: {e}"))?;
            if matches!(state, BoardState::Accepted) && !was_closed {
                r.mutate_sync(StoryEvent::StoryClosed {
                    meta: meta.clone(),
                    id: story_id,
                    reason: Some(rationale.clone()),
                })
                .map_err(|e| format!("#{id} accept-close enqueue failed: {e}"))?;
            }
        }
        TargetState::Retired => {
            r.mutate_sync(StoryEvent::StateTransitioned {
                meta: meta.clone(),
                id: story_id,
                new_state: BoardState::Retired,
                placement: None,
            })
            .map_err(|e| format!("#{id} retire-state enqueue failed: {e}"))?;
            r.mutate_sync(StoryEvent::StoryClosed {
                meta: meta.clone(),
                id: story_id,
                reason: Some(format!("retired: {rationale}")),
            })
            .map_err(|e| format!("#{id} retire enqueue failed: {e}"))?;
        }
    }

    let target_is_icebox = matches!(target, TargetState::Workflow(BoardState::Icebox));
    let target_is_ready = matches!(target, TargetState::Workflow(BoardState::Ready));
    if target_is_icebox && let Some(pos) = position {
        crate::bridge::tools_stories::reorder_icebox_story_via_labels(r, id, pos)?;
    }
    if target_is_ready
        && let Some(pos) = position
        && let Some(iteration_id) = current_iteration_id(&r.iterations())
    {
        crate::bridge::tools_stories::iteration_members::set_member_position(
            r,
            dispatch,
            iteration_id.value(),
            id,
            pos as u32,
        )
        .await
        .map_err(|e| format!("#{id} ready-order (iteration-member) set failed: {e}"))?;
    }

    let body = audit_comment_body(&actor_short, &from_display, &target, position, &rationale);
    r.mutate_sync(StoryEvent::FreeCommentAppended {
        meta,
        id: story_id,
        body,
    })
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "move_story_icebox_label_substrate_tests_1301.rs"]
mod move_story_icebox_label_substrate_tests_1301;

#[cfg(test)]
#[path = "move_story_ready_order_burst_tests.rs"]
mod move_story_ready_order_burst_tests;
