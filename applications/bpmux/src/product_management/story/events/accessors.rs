use super::{EventMeta, StoryEvent};
use crate::product_management::story::StoryId;

impl StoryEvent {
    pub fn meta(&self) -> &EventMeta {
        match self {
            StoryEvent::StoryCreated { meta, .. }
            | StoryEvent::StateTransitioned { meta, .. }
            | StoryEvent::VerdictCast { meta, .. }
            | StoryEvent::FreeCommentAppended { meta, .. }
            | StoryEvent::BodyFetched { meta, .. }
            | StoryEvent::FreeCommentsFetched { meta, .. }
            | StoryEvent::VerdictsFetched { meta, .. }
            | StoryEvent::TagAdded { meta, .. }
            | StoryEvent::TagRemoved { meta, .. }
            | StoryEvent::TagSet { meta, .. }
            | StoryEvent::TagUnset { meta, .. }
            | StoryEvent::IterationSlotSet { meta, .. }
            | StoryEvent::IterationSlotCleared { meta, .. }
            | StoryEvent::StoryClosed { meta, .. }
            | StoryEvent::StoryReopened { meta, .. }
            | StoryEvent::AcceptorAssigned { meta, .. }
            | StoryEvent::ParentEpicSet { meta, .. }
            | StoryEvent::ParentEpicUnset { meta, .. }
            | StoryEvent::CurrentIterationDesignated { meta, .. }
            | StoryEvent::CurrentIterationCleared { meta, .. }
            | StoryEvent::StoryRefreshed { meta, .. }
            | StoryEvent::LabelsRefreshed { meta, .. }
            | StoryEvent::TitleUpdated { meta, .. }
            | StoryEvent::BodyUpdated { meta, .. }
            | StoryEvent::ConflictDetected { meta, .. }
            | StoryEvent::ConflictResolved { meta, .. } => meta,
        }
    }

    pub fn story_id(&self) -> StoryId {
        match self {
            StoryEvent::StoryCreated { id, .. }
            | StoryEvent::StateTransitioned { id, .. }
            | StoryEvent::VerdictCast { id, .. }
            | StoryEvent::FreeCommentAppended { id, .. }
            | StoryEvent::BodyFetched { id, .. }
            | StoryEvent::FreeCommentsFetched { id, .. }
            | StoryEvent::VerdictsFetched { id, .. }
            | StoryEvent::TagAdded { id, .. }
            | StoryEvent::TagRemoved { id, .. }
            | StoryEvent::TagSet { id, .. }
            | StoryEvent::TagUnset { id, .. }
            | StoryEvent::IterationSlotSet { id, .. }
            | StoryEvent::IterationSlotCleared { id, .. }
            | StoryEvent::StoryClosed { id, .. }
            | StoryEvent::StoryReopened { id, .. }
            | StoryEvent::AcceptorAssigned { id, .. }
            | StoryEvent::ParentEpicSet { id, .. }
            | StoryEvent::ParentEpicUnset { id, .. }
            | StoryEvent::CurrentIterationDesignated { id, .. }
            | StoryEvent::CurrentIterationCleared { id, .. }
            | StoryEvent::StoryRefreshed { id, .. }
            | StoryEvent::LabelsRefreshed { id, .. }
            | StoryEvent::TitleUpdated { id, .. }
            | StoryEvent::BodyUpdated { id, .. }
            | StoryEvent::ConflictDetected { id, .. }
            | StoryEvent::ConflictResolved { id, .. } => *id,
        }
    }
}
