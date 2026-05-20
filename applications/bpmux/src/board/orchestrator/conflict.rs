use serde::{Deserialize, Serialize};

use crate::board::domain::Story;
use crate::board::events::BoardEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictKind {
    IdempotentSkip,
    DivergentState,
    DivergentLabel,
    External,
}

pub fn classify_conflict(
    _pre_state: Option<&Story>,
    event: &BoardEvent,
    _current: Option<&Story>,
) -> Option<ConflictKind> {
    match event {
        BoardEvent::TagAdded { .. }
        | BoardEvent::TagRemoved { .. }
        | BoardEvent::StateTransitioned { .. }
        | BoardEvent::StoryClosed { .. }
        | BoardEvent::StoryReopened { .. }
        | BoardEvent::ParentEpicSet { .. }
        | BoardEvent::ParentEpicUnset { .. }
        | BoardEvent::IterationMembershipSet { .. } => None,
        BoardEvent::StoryCreated { .. }
        | BoardEvent::GateCommentAppended { .. }
        | BoardEvent::FreeCommentAppended { .. }
        | BoardEvent::BodyFetched { .. }
        | BoardEvent::FreeCommentsFetched { .. }
        | BoardEvent::GateCommentsFetched { .. }
        | BoardEvent::StoryRefreshed { .. }
        | BoardEvent::LabelsRefreshed { .. }
        | BoardEvent::ConflictDetected { .. }
        | BoardEvent::ConflictResolved { .. } => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolveOrigin {
    DrainAutoReclassify,
    OperatorAction,
}
