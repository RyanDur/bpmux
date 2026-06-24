use std::sync::Arc;

use serde_json::Map;
use serde_json::Value;

use crate::bridge::tools_stories::{KEEP_NONE, reap_workflow_labels};
use crate::product_management::board::domain::StoryState;
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::iteration_label::current_iteration_number;
use crate::product_management::board::{BoardEvent, PeerId, RefreshDriver};

mod target;

use target::{
    TargetState, audit_comment_body, current_state_display, parse_position, require_nonempty,
};

pub(in crate::bridge) async fn handle_move_story(
    args: &Map<String, Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
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

    let actor_role = crate::transport::ws::short_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition = crate::workflow::resolve_transition(
        workflow,
        "move-with-position",
        crate::workflow::GateClass::None,
    )
    .map_err(|e| e.as_json_string())?;
    crate::workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_id().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;

    if position.is_some() {
        let allows_position = matches!(
            target,
            TargetState::Workflow(StoryState::Ready) | TargetState::Workflow(StoryState::Icebox)
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
    let prior =
        Some(crate::bridge::tools_stories::ensure_story_in_canonical_store(r, number).await?);
    let from_display = current_state_display(prior.as_ref()).to_string();
    let was_closed = prior.as_ref().map(|s| s.is_closed).unwrap_or(false);

    let actor_short = crate::transport::ws::short_name(agent_name).to_string();
    let meta = EventMeta {
        actor: actor_short.clone(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    };

    if was_closed && !target.closes_issue() {
        r.mutate_sync(BoardEvent::StoryReopened {
            meta: meta.clone(),
            number,
        })
        .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
    }

    let keep_label = target.as_label().unwrap_or(KEEP_NONE);
    reap_workflow_labels(r.as_ref(), number, &meta, keep_label)?;

    if let Some(label) = target.as_label() {
        let already_labeled = prior
            .as_ref()
            .map(|s| s.raw_labels.iter().any(|l| l == label))
            .unwrap_or(false);
        if !already_labeled {
            r.mutate_sync(BoardEvent::TagAdded {
                meta: meta.clone(),
                number,
                label: label.to_string(),
            })
            .map_err(|e| format!("#{number} tag-add enqueue failed: {e}"))?;
        }
    }

    match target {
        TargetState::Workflow(state) => {
            r.mutate_sync(BoardEvent::StateTransitioned {
                meta: meta.clone(),
                number,
                new_state: state,
            })
            .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
            if matches!(state, StoryState::Accepted) && !was_closed {
                r.mutate_sync(BoardEvent::StoryClosed {
                    meta: meta.clone(),
                    number,
                    reason: Some(rationale.clone()),
                })
                .map_err(|e| format!("#{number} accept-close enqueue failed: {e}"))?;
            }
        }
        TargetState::Retired => {
            r.mutate_sync(BoardEvent::StoryClosed {
                meta: meta.clone(),
                number,
                reason: Some(format!("retired: {rationale}")),
            })
            .map_err(|e| format!("#{number} retire enqueue failed: {e}"))?;
            let already_retired = prior
                .as_ref()
                .map(|s| {
                    s.raw_labels
                        .iter()
                        .any(|l| l.eq_ignore_ascii_case("retired"))
                })
                .unwrap_or(false);
            if !already_retired {
                r.mutate_sync(BoardEvent::TagAdded {
                    meta: meta.clone(),
                    number,
                    label: "retired".to_string(),
                })
                .map_err(|e| format!("#{number} retire-tag enqueue failed: {e}"))?;
            }
        }
    }

    let target_is_icebox = matches!(target, TargetState::Workflow(StoryState::Icebox));
    let target_is_ready = matches!(target, TargetState::Workflow(StoryState::Ready));
    if target_is_icebox && let Some(pos) = position {
        crate::bridge::tools_stories::reorder_icebox_story_via_labels(r, number, pos)?;
    }
    if target_is_ready
        && let Some(pos) = position
        && let Some(iteration_number) = current_iteration_number(&r.all_stories())
    {
        r.mutate_sync(BoardEvent::IterationMembershipSet {
            meta: meta.clone(),
            number,
            iteration_number,
            position: pos as u32,
        })
        .map_err(|e| format!("#{number} ready-order (iteration-membership) enqueue failed: {e}"))?;
    }

    let body = audit_comment_body(&actor_short, &from_display, target, position, &rationale);
    r.mutate_sync(BoardEvent::FreeCommentAppended { meta, number, body })
        .map_err(|e| format!("#{number} audit-comment enqueue failed: {e}"))?;

    let pos_msg = match position {
        Some(p) => format!(" at position {p}"),
        None => String::new(),
    };
    Ok(format!(
        "#{number} moved {from_display} → {to}{pos_msg}",
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
