use crate::board::events::BoardEvent;
use crate::board::sync::SyncError;
use crate::bridge::tracker::OctocrabStoryTracker;

use super::mutation_events::{
    push_current_iteration_cleared, push_current_iteration_designated, push_free_comment_appended,
    push_gate_comment_appended, push_iteration_membership_cleared, push_iteration_membership_set,
    push_outcome_membership_cleared, push_outcome_membership_set, push_parent_epic_set,
    push_parent_epic_unset, push_state_transitioned, push_story_closed, push_story_reopened,
    push_tag_added, push_tag_removed,
};

pub(super) async fn apply_push_event(
    inner: &OctocrabStoryTracker,
    event: &BoardEvent,
) -> Result<Option<BoardEvent>, SyncError> {
    match event {
        BoardEvent::StoryCreated { .. } => Ok(None),
        BoardEvent::StateTransitioned {
            number, new_state, ..
        } => push_state_transitioned(inner, *number, *new_state).await,
        BoardEvent::StoryClosed { number, reason, .. } => {
            push_story_closed(inner, *number, reason).await
        }
        BoardEvent::StoryReopened { number, .. } => push_story_reopened(inner, *number).await,
        BoardEvent::GateCommentAppended {
            meta,
            number,
            gate,
            payload,
        } => push_gate_comment_appended(inner, *number, meta, *gate, payload.as_deref()).await,
        BoardEvent::FreeCommentAppended { number, body, .. } => {
            push_free_comment_appended(inner, *number, body).await
        }
        BoardEvent::TagAdded { number, label, .. } => push_tag_added(inner, *number, label).await,
        BoardEvent::TagRemoved { number, label, .. } => {
            push_tag_removed(inner, *number, label).await
        }
        BoardEvent::ParentEpicSet {
            number,
            epic_number,
            ..
        } => push_parent_epic_set(inner, *number, *epic_number).await,
        BoardEvent::ParentEpicUnset { number, .. } => push_parent_epic_unset(inner, *number).await,
        BoardEvent::IterationMembershipSet {
            number,
            iteration_number,
            position,
            ..
        } => push_iteration_membership_set(inner, *number, *iteration_number, *position).await,
        BoardEvent::IterationMembershipCleared { number, .. } => {
            push_iteration_membership_cleared(inner, *number).await
        }
        BoardEvent::OutcomeMembershipSet {
            number,
            outcome_number,
            position,
            ..
        } => push_outcome_membership_set(inner, *number, *outcome_number, *position).await,
        BoardEvent::OutcomeMembershipCleared { number, .. } => {
            push_outcome_membership_cleared(inner, *number).await
        }
        BoardEvent::CurrentIterationDesignated { number, .. } => {
            push_current_iteration_designated(inner, *number).await
        }
        BoardEvent::CurrentIterationCleared { number, .. } => {
            push_current_iteration_cleared(inner, *number).await
        }
        BoardEvent::BodyFetched { .. }
        | BoardEvent::FreeCommentsFetched { .. }
        | BoardEvent::GateCommentsFetched { .. }
        | BoardEvent::StoryRefreshed { .. }
        | BoardEvent::LabelsRefreshed { .. }
        | BoardEvent::ConflictDetected { .. }
        | BoardEvent::ConflictResolved { .. } => Ok(None),
    }
}
