use serde::{Deserialize, Serialize};

use super::events::StoryEvent;
use crate::product_management::story::Story;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictKind {
    IdempotentSkip,
    DivergentState,
    DivergentLabel,
    External,
}

pub fn classify_conflict(
    _pre_state: Option<&Story>,
    event: &StoryEvent,
    _current: Option<&Story>,
) -> Option<ConflictKind> {
    match event {
        StoryEvent::TagAdded { .. }
        | StoryEvent::TagRemoved { .. }
        | StoryEvent::TagSet { .. }
        | StoryEvent::TagUnset { .. }
        | StoryEvent::BlockedBySet { .. }
        | StoryEvent::BlockedByCleared { .. }
        | StoryEvent::StateTransitioned { .. }
        | StoryEvent::StoryClosed { .. }
        | StoryEvent::StoryReopened { .. }
        | StoryEvent::AcceptorAssigned { .. }
        | StoryEvent::ParentEpicSet { .. }
        | StoryEvent::ParentEpicUnset { .. }
        | StoryEvent::CurrentIterationDesignated { .. }
        | StoryEvent::CurrentIterationCleared { .. } => None,
        StoryEvent::StoryCreated { .. }
        | StoryEvent::GateCommentAppended { .. }
        | StoryEvent::FreeCommentAppended { .. }
        | StoryEvent::BodyFetched { .. }
        | StoryEvent::FreeCommentsFetched { .. }
        | StoryEvent::GateCommentsFetched { .. }
        | StoryEvent::StoryRefreshed { .. }
        | StoryEvent::LabelsRefreshed { .. }
        | StoryEvent::TitleUpdated { .. }
        | StoryEvent::BodyUpdated { .. }
        | StoryEvent::ConflictDetected { .. }
        | StoryEvent::ConflictResolved { .. } => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolveOrigin {
    DrainAutoReclassify,
    OperatorAction,
}
