use std::sync::Arc;

use crate::product_management::epic::{self, EpicAction, EpicState};
use crate::product_management::iteration::{self, IterationAction, IterationState};
use crate::product_management::outcome::{self, OutcomeAction, OutcomeState};
use crate::product_management::story::slice::{self, Stories, StoryAction};
use crate::store::board::{self, BoardAction, BoardPlacements};
use crate::store::ci::{self, CiAction, CiState};
use crate::store::reactions::{self, ReactionsAction, ReactionsState};
use crate::store::roster::{self, RosterAction, RosterState};
use crate::store::sync::SyncAction;

mod board_placement;
mod listeners;
mod owner_park;
pub use board_placement::{BoardPlacementPersister, BoardPlacementReconciler};
pub use listeners::*;
pub use owner_park::OwnerParkListener;

#[cfg(test)]
mod board_placement_tests;

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
}

impl AppState {
    pub fn with_board(board: BoardPlacements) -> Self {
        Self {
            board: Arc::new(board),
            ..Default::default()
        }
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

pub fn app_reduce(state: &AppState, action: &AppAction) -> AppState {
    match action {
        AppAction::Roster(action) => match roster::reduce(&state.roster, action) {
            Some(roster) => AppState {
                roster: Arc::new(roster),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::Reactions(action) => match reactions::reduce(&state.reactions, action) {
            Some(reactions) => AppState {
                reactions: Arc::new(reactions),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::Ci(action) => match ci::reduce(&state.ci, action) {
            Some(ci) => AppState {
                ci: Arc::new(ci),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::Sync(_) => state.clone(),
        AppAction::Outcome(action) => match outcome::reduce(&state.outcomes, action) {
            Some(outcomes) => AppState {
                outcomes: Arc::new(outcomes),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::Iteration(action) => match iteration::reduce(&state.iterations, action) {
            Some(iterations) => AppState {
                iterations: Arc::new(iterations),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::Epic(action) => match epic::reduce(&state.epics, action) {
            Some(epics) => AppState {
                epics: Arc::new(epics),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::Story(action) => {
            let stories = Arc::new(slice::reduce(&state.stories, action));
            let board = board::react_to_story(&state.board, action)
                .map(Arc::new)
                .unwrap_or_else(|| state.board.clone());
            AppState {
                stories,
                board,
                ..state.clone()
            }
        }
        AppAction::Board(action) => match board::reduce(&state.board, action) {
            Some(board) => AppState {
                board: Arc::new(board),
                ..state.clone()
            },
            None => state.clone(),
        },
    }
}

pub(super) fn validate_and_log(action: &AppAction) {
    let label = match action {
        AppAction::Ci(CiAction::StartPolling) => "Ci.StartPolling",
        AppAction::Ci(CiAction::StopPolling) => "Ci.StopPolling",
        AppAction::Ci(CiAction::Updated(_)) => "Ci.Updated",
        AppAction::Sync(SyncAction::StartIngestPolling) => "Sync.StartIngestPolling",
        AppAction::Sync(SyncAction::StopIngestPolling) => "Sync.StopIngestPolling",
        AppAction::Sync(SyncAction::StartHydrationPolling) => "Sync.StartHydrationPolling",
        AppAction::Sync(SyncAction::StopHydrationPolling) => "Sync.StopHydrationPolling",
        AppAction::Sync(SyncAction::StartUpdateGithub { .. }) => "Sync.StartUpdateGithub",
        AppAction::Sync(SyncAction::StopUpdateGithub { .. }) => "Sync.StopUpdateGithub",
        AppAction::Sync(SyncAction::MutationConfirmed { .. }) => "Sync.MutationConfirmed",
        AppAction::Sync(SyncAction::MutationRolledBack { .. }) => "Sync.MutationRolledBack",
        AppAction::Reactions(_) => "Reactions",
        AppAction::Roster(_) => "Roster",
        AppAction::Outcome(_) => "Outcome",
        AppAction::Iteration(_) => "Iteration",
        AppAction::Epic(_) => "Epic",
        AppAction::Story(_) => "Story",
        AppAction::Board(_) => "Board",
    };
    crate::diag::log_diag(&format!("store: dispatch {label}"));
}
