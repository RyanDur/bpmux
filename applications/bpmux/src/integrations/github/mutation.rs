use crate::bridge::tracker::OctocrabStoryTracker;
use crate::product_management::board::events::BoardEvent;
use crate::product_management::board::sync::SyncError;

use super::mutation_events::{
    push_acceptor_assigned, push_current_iteration_cleared, push_current_iteration_designated,
    push_free_comment_appended, push_gate_comment_appended, push_iteration_membership_cleared,
    push_iteration_membership_set, push_parent_epic_set, push_parent_epic_unset,
    push_state_transitioned, push_story_closed, push_story_reopened, push_tag_added,
    push_tag_removed,
};

pub(super) async fn apply_push_event(
    inner: &OctocrabStoryTracker,
    event: &BoardEvent,
) -> Result<Option<BoardEvent>, SyncError> {
    match event {
        BoardEvent::StoryCreated { .. } => Ok(None),
        BoardEvent::StateTransitioned { id, new_state, .. } => {
            push_state_transitioned(inner, id.value(), *new_state).await
        }
        BoardEvent::StoryClosed { id, reason, .. } => {
            push_story_closed(inner, id.value(), reason).await
        }
        BoardEvent::StoryReopened { id, .. } => push_story_reopened(inner, id.value()).await,
        BoardEvent::GateCommentAppended {
            meta,
            id,
            gate,
            payload,
        } => push_gate_comment_appended(inner, id.value(), meta, *gate, payload.as_deref()).await,
        BoardEvent::FreeCommentAppended { id, body, .. } => {
            push_free_comment_appended(inner, id.value(), body).await
        }
        BoardEvent::TagAdded { id, label, .. } => push_tag_added(inner, id.value(), label).await,
        BoardEvent::TagRemoved { id, label, .. } => {
            push_tag_removed(inner, id.value(), label).await
        }
        BoardEvent::AcceptorAssigned { id, acceptor, .. } => {
            push_acceptor_assigned(inner, id.value(), *acceptor).await
        }
        BoardEvent::ParentEpicSet { id, epic_id, .. } => {
            push_parent_epic_set(inner, id.value(), *epic_id).await
        }
        BoardEvent::ParentEpicUnset { id, .. } => push_parent_epic_unset(inner, id.value()).await,
        BoardEvent::IterationMembershipSet {
            id,
            iteration_id,
            position,
            ..
        } => push_iteration_membership_set(inner, id.value(), *iteration_id, *position).await,
        BoardEvent::IterationMembershipCleared { id, .. } => {
            push_iteration_membership_cleared(inner, id.value()).await
        }
        BoardEvent::CurrentIterationDesignated { id, .. } => {
            push_current_iteration_designated(inner, id.value()).await
        }
        BoardEvent::CurrentIterationCleared { id, .. } => {
            push_current_iteration_cleared(inner, id.value()).await
        }
        BoardEvent::BodyFetched { .. }
        | BoardEvent::FreeCommentsFetched { .. }
        | BoardEvent::GateCommentsFetched { .. }
        | BoardEvent::StoryRefreshed { .. }
        | BoardEvent::LabelsRefreshed { .. }
        | BoardEvent::TitleUpdated { .. }
        | BoardEvent::BodyUpdated { .. }
        | BoardEvent::ConflictDetected { .. }
        | BoardEvent::ConflictResolved { .. } => Ok(None),
    }
}
