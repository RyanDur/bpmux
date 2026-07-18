use std::sync::Arc;

use crate::board;
use crate::ci;
use crate::reactions;
use crate::roster;
use product_management::epic;
use product_management::evidence;
use product_management::iteration;
use product_management::outcome;
use product_management::story::slice;

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
        AppAction::Evidence(action) => match evidence::reduce(&state.evidence, action) {
            Some(evidence) => AppState {
                evidence: Arc::new(evidence),
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
        AppAction::Sync(action) => match crate::sync::reduce(&state.sync, action) {
            Some(sync) => AppState {
                sync: Arc::new(sync),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::SyncHealth(action) => {
            match crate::sync_health::reduce(&state.sync_health, action) {
                Some(sync_health) => AppState {
                    sync_health: Arc::new(sync_health),
                    ..state.clone()
                },
                None => state.clone(),
            }
        }
        AppAction::Commitments(action) => {
            match crate::commitments::reduce(&state.commitments, action) {
                Some(commitments) => AppState {
                    commitments: Arc::new(commitments),
                    ..state.clone()
                },
                None => state.clone(),
            }
        }
        AppAction::Routing(action) => match crate::routing::reduce(&state.routing, action) {
            Some(routing) => AppState {
                routing: Arc::new(routing),
                ..state.clone()
            },
            None => state.clone(),
        },
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
                product_management::history::react_to_story(&state.shared_history, action)
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
        AppAction::SharedHistory(action) => {
            match product_management::history::reduce(&state.shared_history, action) {
                Some(shared_history) => AppState {
                    shared_history: Arc::new(shared_history),
                    ..state.clone()
                },
                None => state.clone(),
            }
        }
        AppAction::OwnerSession(action) => {
            match owner_session::reduce(&state.owner_session, action) {
                Some(owner_session) => AppState {
                    owner_session: Arc::new(owner_session),
                    ..state.clone()
                },
                None => state.clone(),
            }
        }
    }
}
