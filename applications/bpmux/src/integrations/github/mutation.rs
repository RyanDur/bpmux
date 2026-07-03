use crate::bridge::tracker::OctocrabStoryTracker;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::events::StoryEvent;
use crate::product_management::board::sync::SyncError;

use super::mutation_events::{
    push_acceptor_assigned, push_blocked_by_cleared, push_blocked_by_set,
    push_current_iteration_cleared, push_current_iteration_designated, push_free_comment_appended,
    push_gate_comment_appended, push_iteration_slot_cleared, push_iteration_slot_set,
    push_parent_epic_set, push_parent_epic_unset, push_state_transitioned, push_story_closed,
    push_story_reopened, push_tag_added, push_tag_removed,
};

pub(super) async fn apply_push_event(
    inner: &OctocrabStoryTracker,
    event: &StoryEvent,
) -> Result<Option<StoryEvent>, SyncError> {
    match event {
        StoryEvent::StoryCreated { .. } => Ok(None),
        StoryEvent::StateTransitioned { id, new_state, .. } => {
            push_state_transitioned(inner, id.value(), new_state.clone()).await
        }
        StoryEvent::StoryClosed { id, reason, .. } => {
            push_story_closed(inner, id.value(), reason).await
        }
        StoryEvent::StoryReopened { id, .. } => push_story_reopened(inner, id.value()).await,
        StoryEvent::GateCommentAppended {
            meta,
            id,
            gate,
            payload,
        } => push_gate_comment_appended(inner, id.value(), meta, *gate, payload.as_deref()).await,
        StoryEvent::FreeCommentAppended { id, body, .. } => {
            push_free_comment_appended(inner, id.value(), body).await
        }
        StoryEvent::TagAdded { id, label, .. } => push_tag_added(inner, id.value(), label).await,
        StoryEvent::TagRemoved { id, label, .. } => {
            push_tag_removed(inner, id.value(), label).await
        }
        StoryEvent::TagSet { id, tag, .. } => {
            push_tag_added(inner, id.value(), GhLabelCodec.encode_tag(*tag)).await
        }
        StoryEvent::TagUnset { id, tag, .. } => {
            push_tag_removed(inner, id.value(), GhLabelCodec.encode_tag(*tag)).await
        }
        StoryEvent::BlockedBySet { id, blocker, .. } => {
            push_blocked_by_set(inner, id.value(), blocker.value()).await
        }
        StoryEvent::BlockedByCleared { id, .. } => push_blocked_by_cleared(inner, id.value()).await,
        StoryEvent::IterationSlotSet {
            id, name, position, ..
        } => push_iteration_slot_set(inner, id.value(), name, *position).await,
        StoryEvent::IterationSlotCleared { id, .. } => {
            push_iteration_slot_cleared(inner, id.value()).await
        }
        StoryEvent::AcceptorAssigned { id, acceptor, .. } => {
            push_acceptor_assigned(inner, id.value(), *acceptor).await
        }
        StoryEvent::ParentEpicSet { id, epic_id, .. } => {
            push_parent_epic_set(inner, id.value(), *epic_id).await
        }
        StoryEvent::ParentEpicUnset { id, .. } => push_parent_epic_unset(inner, id.value()).await,
        StoryEvent::CurrentIterationDesignated { id, .. } => {
            push_current_iteration_designated(inner, id.value()).await
        }
        StoryEvent::CurrentIterationCleared { id, .. } => {
            push_current_iteration_cleared(inner, id.value()).await
        }
        StoryEvent::BodyFetched { .. }
        | StoryEvent::FreeCommentsFetched { .. }
        | StoryEvent::GateCommentsFetched { .. }
        | StoryEvent::StoryRefreshed { .. }
        | StoryEvent::LabelsRefreshed { .. }
        | StoryEvent::TitleUpdated { .. }
        | StoryEvent::BodyUpdated { .. }
        | StoryEvent::ConflictDetected { .. }
        | StoryEvent::ConflictResolved { .. } => Ok(None),
    }
}
