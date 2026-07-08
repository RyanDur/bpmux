use std::sync::Arc;

use crate::product_management::epic::{EpicAction, EpicState};
use crate::product_management::history::{SharedHistory, SharedHistoryAction};
use crate::product_management::iteration::{IterationAction, IterationState};
use crate::product_management::outcome::{OutcomeAction, OutcomeState};
use crate::product_management::story::slice::{Stories, StoryAction};
use crate::store::board::history::PlacementHistory;
use crate::store::board::{BoardAction, BoardPlacements};
use crate::store::ci::{CiAction, CiState};
use crate::store::reactions::{ReactionsAction, ReactionsState};
use crate::store::roster::{RosterAction, RosterState};
use crate::store::sync::{SyncAction, SyncState};

mod board_placement;
mod history;
mod listeners;
mod owner_park;
mod reactions;
mod reduce;
pub use board_placement::{BoardPlacementPersister, BoardPlacementReconciler};
pub use history::{LocalHistoryPersister, SharedHistoryPersister};
pub use listeners::*;
pub use owner_park::OwnerParkListener;
pub use reactions::ReactionsPersister;
pub use reduce::app_reduce;
pub(super) use reduce::validate_and_log;

#[cfg(test)]
mod board_placement_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod owner_park_tests;
#[cfg(test)]
mod reactions_tests;

#[derive(Default, Clone)]
pub struct AppState {
    pub roster: Arc<RosterState>,
    pub reactions: Arc<ReactionsState>,
    pub ci: Arc<CiState>,
    pub outcomes: Arc<OutcomeState>,
    pub iterations: Arc<IterationState>,
    pub epics: Arc<EpicState>,
    pub stories: Arc<Stories>,
    pub board: Arc<BoardPlacements>,
    pub shared_history: Arc<SharedHistory>,
    pub local_history: Arc<PlacementHistory>,
    pub sync: Arc<SyncState>,
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

    pub fn with_shared_history(mut self, shared_history: SharedHistory) -> Self {
        self.shared_history = Arc::new(shared_history);
        self
    }

    pub fn with_local_history(mut self, local_history: PlacementHistory) -> Self {
        self.local_history = Arc::new(local_history);
        self
    }

    pub fn with_outbox(mut self, writes: Vec<crate::sync::QueuedWrite>) -> Self {
        self.sync = Arc::new(SyncState {
            queue: writes.into_iter().collect(),
        });
        self
    }
}

#[derive(Clone)]
pub enum AppAction {
    Roster(RosterAction),
    Reactions(ReactionsAction),
    Ci(CiAction),
    Sync(SyncAction),
    Outcome(OutcomeAction),
    Iteration(IterationAction),
    Epic(EpicAction),
    Story(StoryAction),
    Board(BoardAction),
    SharedHistory(SharedHistoryAction),
}

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
