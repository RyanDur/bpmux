use serde::{Deserialize, Serialize};

use super::events::{StoryEvent, StoryEventKind};
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
    match &event.kind {
        StoryEventKind::TagAdded { .. }
        | StoryEventKind::TagRemoved { .. }
        | StoryEventKind::TagSet { .. }
        | StoryEventKind::TagUnset { .. }
        | StoryEventKind::StateTransitioned { .. }
        | StoryEventKind::StoryClosed { .. }
        | StoryEventKind::StoryReopened
        | StoryEventKind::AcceptorAssigned { .. }
        | StoryEventKind::ParentEpicSet { .. }
        | StoryEventKind::ParentEpicUnset
        | StoryEventKind::CurrentIterationDesignated
        | StoryEventKind::CurrentIterationCleared => None,
        StoryEventKind::StoryCreated { .. }
        | StoryEventKind::VerdictCast { .. }
        | StoryEventKind::FreeCommentAppended { .. }
        | StoryEventKind::BodyFetched { .. }
        | StoryEventKind::FreeCommentsFetched { .. }
        | StoryEventKind::VerdictsFetched { .. }
        | StoryEventKind::StoryRefreshed { .. }
        | StoryEventKind::LabelsRefreshed { .. }
        | StoryEventKind::TitleUpdated { .. }
        | StoryEventKind::BodyUpdated { .. }
        | StoryEventKind::ConflictDetected { .. }
        | StoryEventKind::ConflictResolved { .. } => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolveOrigin {
    DrainAutoReclassify,
    OperatorAction,
}
