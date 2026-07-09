use crate::routing::cursor_time::later_cursor;

use super::{RoutingAction, RoutingState};

pub fn reduce(state: &RoutingState, action: &RoutingAction) -> Option<RoutingState> {
    match action {
        RoutingAction::Registered { user_id, display } => {
            let mut identity = state.identity.clone();
            identity.register(user_id, display);
            Some(RoutingState { identity })
        }
        RoutingAction::TerminalLinked {
            user_id,
            terminal_id,
        } => {
            let mut identity = state.identity.clone();
            identity.link_terminal(user_id, terminal_id);
            Some(RoutingState { identity })
        }
        RoutingAction::Renamed { user_id, display } => {
            let mut identity = state.identity.clone();
            identity.rename(user_id, display);
            Some(RoutingState { identity })
        }
        RoutingAction::CursorAdvanced { user_id, cursor } => {
            let current = state.identity.cursor(user_id);
            match later_cursor(current.as_deref(), Some(cursor)) {
                Some(later) if current.as_deref() != Some(later.as_str()) => {
                    let mut identity = state.identity.clone();
                    identity.advance_cursor(user_id, &later);
                    Some(RoutingState { identity })
                }
                _ => None,
            }
        }
        RoutingAction::CursorAnchored { user_id, cursor } => {
            let mut identity = state.identity.clone();
            identity.advance_cursor(user_id, cursor);
            Some(RoutingState { identity })
        }
        RoutingAction::Reaped { display } => {
            let mut identity = state.identity.clone();
            identity.reap_unclaimed_duplicates_of(display);
            Some(RoutingState { identity })
        }
    }
}
