use std::sync::Arc;

use crate::board::history::PlacementHistory;
use crate::board::{BoardAction, BoardPlacements};
use crate::ci::{CiAction, CiState};
use crate::commitments::{CommitmentsAction, CommitmentsState};
use crate::reactions::{ReactionsAction, ReactionsState};
use crate::roster::{RosterAction, RosterState};
use crate::routing::{RoutingAction, RoutingState};
use crate::sync::{SyncAction, SyncState};
use crate::sync_health::{SyncHealthAction, SyncHealthState};
use owner_session::{OwnerSessionAction, OwnerSessionState};
use product_management::epic::{EpicAction, EpicState};
use product_management::evidence::{EvidenceAction, EvidenceState};
use product_management::history::{SharedHistory, SharedHistoryAction};
use product_management::iteration::{IterationAction, IterationState};
use product_management::outcome::{OutcomeAction, OutcomeState};
use product_management::story::slice::{Stories, StoryAction};

mod action_conversions;
mod board_placement;
mod ci_runs;
mod dispatch_log;
mod evidence;
mod history;
mod listeners;
mod owner_park;
mod reactions;
mod reduce;
mod routing;
mod transitions;
pub use board_placement::{BoardPlacementPersister, BoardPlacementReconciler};
pub use ci_runs::CiRunsPersister;
pub(super) use dispatch_log::validate_and_log;
pub use evidence::EvidencePersister;
pub use history::{LocalHistoryPersister, SharedHistoryPersister};
pub use listeners::*;
pub use owner_park::OwnerParkListener;
pub use reactions::ReactionsPersister;
pub use reduce::app_reduce;
pub use routing::RoutingPersister;
pub use transitions::TransitionPersister;

#[cfg(test)]
mod board_placement_tests;
#[cfg(test)]
mod evidence_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod owner_park_tests;
#[cfg(test)]
mod reactions_tests;
#[cfg(test)]
mod routing_tests;

#[derive(Default, Clone)]
pub struct AppState {
    pub roster: Arc<RosterState>,
    pub reactions: Arc<ReactionsState>,
    pub evidence: Arc<EvidenceState>,
    pub ci: Arc<CiState>,
    pub outcomes: Arc<OutcomeState>,
    pub iterations: Arc<IterationState>,
    pub epics: Arc<EpicState>,
    pub stories: Arc<Stories>,
    pub board: Arc<BoardPlacements>,
    pub shared_history: Arc<SharedHistory>,
    pub local_history: Arc<PlacementHistory>,
    pub sync: Arc<SyncState>,
    pub sync_health: Arc<SyncHealthState>,
    pub commitments: Arc<CommitmentsState>,
    pub routing: Arc<RoutingState>,
    pub owner_session: Arc<OwnerSessionState>,
}

impl AppState {
    pub fn with_board(board: BoardPlacements) -> Self {
        Self {
            board: Arc::new(board),
            ..Default::default()
        }
    }

    pub fn with_reactions(mut self, reactions: ReactionsState) -> Self {
        self.reactions = Arc::new(reactions);
        self
    }

    pub fn with_evidence(mut self, evidence: EvidenceState) -> Self {
        self.evidence = Arc::new(evidence);
        self
    }

    pub fn with_shared_history(mut self, shared_history: SharedHistory) -> Self {
        self.shared_history = Arc::new(shared_history);
        self
    }

    pub fn with_local_history(mut self, local_history: PlacementHistory) -> Self {
        self.local_history = Arc::new(local_history);
        self
    }

    pub fn with_routing(mut self, routing: RoutingState) -> Self {
        self.routing = Arc::new(routing);
        self
    }

    pub fn with_ci(mut self, ci: CiState) -> Self {
        self.ci = Arc::new(ci);
        self
    }

    pub fn with_outbox(mut self, writes: Vec<source_sync::QueuedWrite>) -> Self {
        let next_mutation_id = writes.iter().map(|write| write.id.0).max().unwrap_or(0) + 1;
        self.sync = Arc::new(SyncState {
            queue: writes.into_iter().collect(),
            next_mutation_id,
        });
        self
    }
}

#[derive(Clone)]
pub enum AppAction {
    Roster(RosterAction),
    Reactions(ReactionsAction),
    Evidence(EvidenceAction),
    Ci(CiAction),
    Sync(SyncAction),
    SyncHealth(SyncHealthAction),
    Commitments(CommitmentsAction),
    Routing(RoutingAction),
    Outcome(OutcomeAction),
    Iteration(IterationAction),
    Epic(EpicAction),
    Story(StoryAction),
    Board(BoardAction),
    SharedHistory(SharedHistoryAction),
    OwnerSession(OwnerSessionAction),
}
