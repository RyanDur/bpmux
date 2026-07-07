use crate::bridge::tracker::OctocrabStoryTracker;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::product_management::label_codec::LabelCodec;
use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::sync::SyncError;

use super::mutation_events::{
    push_acceptor_assigned, push_body_updated, push_current_iteration_cleared,
    push_current_iteration_designated, push_free_comment_appended, push_iteration_slot_cleared,
    push_iteration_slot_set, push_parent_epic_set, push_parent_epic_unset, push_state_transitioned,
    push_story_closed, push_story_reopened, push_tag_added, push_tag_removed, push_title_updated,
    push_verdict_cast,
};

pub(super) async fn apply_push_event(
    inner: &OctocrabStoryTracker,
    event: &StoryEvent,
    context: &crate::sync::PushContext,
) -> Result<Option<StoryEvent>, SyncError> {
    match &event.kind {
        StoryEventKind::StoryCreated { .. } => Ok(None),
        StoryEventKind::StateTransitioned { new_state, .. } => {
            push_state_transitioned(
                inner,
                event.story_id().value(),
                new_state.clone(),
                context.member_of_current_iteration,
            )
            .await
        }
        StoryEventKind::StoryClosed { reason } => {
            push_story_closed(inner, event.story_id().value(), reason).await
        }
        StoryEventKind::StoryReopened => push_story_reopened(inner, event.story_id().value()).await,
        StoryEventKind::VerdictCast { verdict, payload } => {
            push_verdict_cast(
                inner,
                event.story_id().value(),
                event.meta(),
                verdict,
                payload.as_deref(),
            )
            .await
        }
        StoryEventKind::FreeCommentAppended { body } => {
            push_free_comment_appended(inner, event.story_id().value(), body).await
        }
        StoryEventKind::TagAdded { label } => {
            push_tag_added(inner, event.story_id().value(), label).await
        }
        StoryEventKind::TagRemoved { label } => {
            push_tag_removed(inner, event.story_id().value(), label).await
        }
        StoryEventKind::TagSet { tag } => {
            push_tag_added(
                inner,
                event.story_id().value(),
                GhLabelCodec.encode_tag(*tag),
            )
            .await
        }
        StoryEventKind::TagUnset { tag } => {
            push_tag_removed(
                inner,
                event.story_id().value(),
                GhLabelCodec.encode_tag(*tag),
            )
            .await
        }
        StoryEventKind::IterationSlotSet { name, position } => {
            push_iteration_slot_set(inner, event.story_id().value(), name, *position).await
        }
        StoryEventKind::IterationSlotCleared => {
            push_iteration_slot_cleared(inner, event.story_id().value()).await
        }
        StoryEventKind::AcceptorAssigned { acceptor } => {
            push_acceptor_assigned(inner, event.story_id().value(), *acceptor).await
        }
        StoryEventKind::ParentEpicSet { epic_id } => {
            push_parent_epic_set(inner, event.story_id().value(), *epic_id).await
        }
        StoryEventKind::ParentEpicUnset => {
            push_parent_epic_unset(inner, event.story_id().value()).await
        }
        StoryEventKind::CurrentIterationDesignated => {
            push_current_iteration_designated(inner, event.story_id().value()).await
        }
        StoryEventKind::CurrentIterationCleared => {
            push_current_iteration_cleared(inner, event.story_id().value()).await
        }
        StoryEventKind::BodyUpdated { body } => {
            push_body_updated(inner, event.story_id().value(), body).await
        }
        StoryEventKind::TitleUpdated { title } => {
            push_title_updated(inner, event.story_id().value(), title).await
        }
        StoryEventKind::BodyFetched { .. }
        | StoryEventKind::FreeCommentsFetched { .. }
        | StoryEventKind::VerdictsFetched { .. }
        | StoryEventKind::StoryRefreshed { .. }
        | StoryEventKind::LabelsRefreshed { .. }
        | StoryEventKind::ConflictDetected { .. }
        | StoryEventKind::ConflictResolved { .. } => Ok(None),
    }
}
