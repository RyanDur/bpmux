use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::{StoryId, Work};
use sync::SyncOrchestrator;

use super::target::TargetState;

pub(super) fn emit_reopen_and_label(
    r: &SyncOrchestrator,
    meta: &EventMeta,
    story_id: StoryId,
    target: &TargetState,
    prior: Option<&product_management::story::Story>,
    was_closed: bool,
) -> Result<(), String> {
    let id = u64::from(story_id);
    if was_closed && !target.closes_issue() {
        r.mutate(StoryEvent::new(
            meta.clone(),
            story_id,
            StoryEventKind::StoryReopened,
        ))
        .map_err(|e| format!("#{id} reopen enqueue failed: {e}"))?;
    }
    let Some(label) = target.as_label() else {
        return Ok(());
    };
    let target_state = target.workflow_state().unwrap_or(Work::Retired);
    let already_labeled = prior
        .map(|story| story.state() == target_state)
        .unwrap_or(false);
    if !already_labeled {
        r.mutate(StoryEvent::new(
            meta.clone(),
            story_id,
            StoryEventKind::TagAdded {
                label: label.to_string(),
            },
        ))
        .map_err(|e| format!("#{id} tag-add enqueue failed: {e}"))?;
    }
    Ok(())
}

pub(super) fn emit_transition(
    r: &SyncOrchestrator,
    meta: &EventMeta,
    story_id: StoryId,
    target: &TargetState,
    target_str: &str,
    was_closed: bool,
    rationale: &str,
) -> Result<(), String> {
    let id = u64::from(story_id);
    match target {
        TargetState::Icebox | TargetState::Ready | TargetState::Workflow(_) => {
            let state = target
                .workflow_state()
                .ok_or_else(|| format!("#{id} move target has no workflow state"))?;
            r.mutate(StoryEvent::new(
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
                r.mutate(StoryEvent::new(
                    meta.clone(),
                    story_id,
                    StoryEventKind::StoryClosed {
                        reason: Some(rationale.to_string()),
                    },
                ))
                .map_err(|e| format!("#{id} accept-close enqueue failed: {e}"))?;
            }
        }
        TargetState::Retired => {
            r.mutate(StoryEvent::new(
                meta.clone(),
                story_id,
                StoryEventKind::StateTransitioned {
                    new_state: Work::Retired,
                    placement: None,
                },
            ))
            .map_err(|e| format!("#{id} retire-state enqueue failed: {e}"))?;
            r.mutate(StoryEvent::new(
                meta.clone(),
                story_id,
                StoryEventKind::StoryClosed {
                    reason: Some(format!("retired: {rationale}")),
                },
            ))
            .map_err(|e| format!("#{id} retire enqueue failed: {e}"))?;
        }
    }
    Ok(())
}
