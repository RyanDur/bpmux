use serde::{Deserialize, Serialize};

use crate::product_management::board::events::BoardEvent;
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
    event: &BoardEvent,
    _current: Option<&Story>,
) -> Option<ConflictKind> {
    match event {
        BoardEvent::TagAdded { .. }
        | BoardEvent::TagRemoved { .. }
        | BoardEvent::StateTransitioned { .. }
        | BoardEvent::StoryClosed { .. }
        | BoardEvent::StoryReopened { .. }
        | BoardEvent::AcceptorAssigned { .. }
        | BoardEvent::ParentEpicSet { .. }
        | BoardEvent::ParentEpicUnset { .. }
        | BoardEvent::IterationMembershipSet { .. }
        | BoardEvent::IterationMembershipCleared { .. }
        | BoardEvent::CurrentIterationDesignated { .. }
        | BoardEvent::CurrentIterationCleared { .. } => None,
        BoardEvent::StoryCreated { .. }
        | BoardEvent::GateCommentAppended { .. }
        | BoardEvent::FreeCommentAppended { .. }
        | BoardEvent::BodyFetched { .. }
        | BoardEvent::FreeCommentsFetched { .. }
        | BoardEvent::GateCommentsFetched { .. }
        | BoardEvent::StoryRefreshed { .. }
        | BoardEvent::LabelsRefreshed { .. }
        | BoardEvent::TitleUpdated { .. }
        | BoardEvent::BodyUpdated { .. }
        | BoardEvent::ConflictDetected { .. }
        | BoardEvent::ConflictResolved { .. } => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolveOrigin {
    DrainAutoReclassify,
    OperatorAction,
}
