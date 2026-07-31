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

macro_rules! patched {
    ($state:expr, $field:ident, $reduced:expr) => {
        match $reduced {
            Some(value) => AppState {
                $field: Arc::new(value),
                ..$state.clone()
            },
            None => $state.clone(),
        }
    };
}

#[expect(
    clippy::cognitive_complexity,
    reason = "flat exhaustive dispatch — the root reducer routes each action family to its slice with no nesting; clippy scores the patched! expansion per arm where the Sonar spec charges the match once"
)]
pub fn app_reduce(state: &AppState, action: &AppAction) -> AppState {
    match action {
        AppAction::Roster(action) => patched!(state, roster, roster::reduce(&state.roster, action)),
        AppAction::Reactions(action) => {
            patched!(
                state,
                reactions,
                reactions::reduce(&state.reactions, action)
            )
        }
        AppAction::Evidence(action) => {
            patched!(state, evidence, evidence::reduce(&state.evidence, action))
        }
        AppAction::Ci(action) => patched!(state, ci, ci::reduce(&state.ci, action)),
        AppAction::Sync(action) => patched!(state, sync, crate::sync::reduce(&state.sync, action)),
        AppAction::SyncHealth(action) => {
            patched!(
                state,
                sync_health,
                crate::sync_health::reduce(&state.sync_health, action)
            )
        }
        AppAction::Commitments(action) => {
            patched!(
                state,
                commitments,
                crate::commitments::reduce(&state.commitments, action)
            )
        }
        AppAction::Routing(action) => {
            patched!(
                state,
                routing,
                crate::routing::reduce(&state.routing, action)
            )
        }
        AppAction::Outcome(action) => {
            patched!(state, outcomes, outcome::reduce(&state.outcomes, action))
        }
        AppAction::Iteration(action) => {
            patched!(
                state,
                iterations,
                iteration::reduce(&state.iterations, action)
            )
        }
        AppAction::Epic(action) => patched!(state, epics, epic::reduce(&state.epics, action)),
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
        AppAction::Board(action) => patched!(state, board, board::reduce(&state.board, action)),
        AppAction::SharedHistory(action) => {
            patched!(
                state,
                shared_history,
                product_management::history::reduce(&state.shared_history, action)
            )
        }
        AppAction::OwnerSession(action) => {
            patched!(
                state,
                owner_session,
                owner_session::reduce(&state.owner_session, action)
            )
        }
        AppAction::ClientConnected(_) => state.clone(),
    }
}
