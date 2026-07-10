use super::AppAction;
use crate::product_management::epic::EpicAction;
use crate::product_management::evidence::EvidenceAction;
use crate::product_management::history::SharedHistoryAction;
use crate::product_management::iteration::IterationAction;
use crate::product_management::outcome::OutcomeAction;
use crate::product_management::story::slice::StoryAction;
use crate::store::board::BoardAction;
use crate::store::ci::CiAction;
use crate::store::commitments::CommitmentsAction;
use crate::store::owner_session::OwnerSessionAction;
use crate::store::reactions::ReactionsAction;
use crate::store::roster::RosterAction;
use crate::store::routing::RoutingAction;
use crate::store::sync::SyncAction;
use crate::store::sync_health::SyncHealthAction;

impl From<RosterAction> for AppAction {
    fn from(action: RosterAction) -> Self {
        Self::Roster(action)
    }
}

impl From<ReactionsAction> for AppAction {
    fn from(action: ReactionsAction) -> Self {
        Self::Reactions(action)
    }
}

impl From<EvidenceAction> for AppAction {
    fn from(action: EvidenceAction) -> Self {
        Self::Evidence(action)
    }
}

impl From<CiAction> for AppAction {
    fn from(action: CiAction) -> Self {
        Self::Ci(action)
    }
}

impl From<SyncAction> for AppAction {
    fn from(action: SyncAction) -> Self {
        Self::Sync(action)
    }
}

impl From<SyncHealthAction> for AppAction {
    fn from(action: SyncHealthAction) -> Self {
        Self::SyncHealth(action)
    }
}

impl From<CommitmentsAction> for AppAction {
    fn from(action: CommitmentsAction) -> Self {
        Self::Commitments(action)
    }
}

impl From<RoutingAction> for AppAction {
    fn from(action: RoutingAction) -> Self {
        Self::Routing(action)
    }
}

impl From<OutcomeAction> for AppAction {
    fn from(action: OutcomeAction) -> Self {
        Self::Outcome(action)
    }
}

impl From<IterationAction> for AppAction {
    fn from(action: IterationAction) -> Self {
        Self::Iteration(action)
    }
}

impl From<EpicAction> for AppAction {
    fn from(action: EpicAction) -> Self {
        Self::Epic(action)
    }
}

impl From<StoryAction> for AppAction {
    fn from(action: StoryAction) -> Self {
        Self::Story(action)
    }
}

impl From<BoardAction> for AppAction {
    fn from(action: BoardAction) -> Self {
        Self::Board(action)
    }
}

impl From<SharedHistoryAction> for AppAction {
    fn from(action: SharedHistoryAction) -> Self {
        Self::SharedHistory(action)
    }
}

impl From<OwnerSessionAction> for AppAction {
    fn from(action: OwnerSessionAction) -> Self {
        Self::OwnerSession(action)
    }
}
