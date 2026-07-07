use std::sync::Arc;

use crate::product_management::epic;
use crate::product_management::iteration;
use crate::product_management::outcome;
use crate::product_management::story::slice;
use crate::store::board;
use crate::store::ci::{self, CiAction};
use crate::store::reactions;
use crate::store::roster;
use crate::store::sync::SyncAction;

use super::{AppAction, AppState};

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
            let shared_history =
                crate::product_management::history::react_to_story(&state.shared_history, action)
                    .map(Arc::new)
                    .unwrap_or_else(|| state.shared_history.clone());
            let local_history = board::history::react_to_story(&state.local_history, action)
                .map(Arc::new)
                .unwrap_or_else(|| state.local_history.clone());
            AppState {
                stories,
                board,
                shared_history,
                local_history,
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

pub fn validate_and_log(action: &AppAction) {
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
