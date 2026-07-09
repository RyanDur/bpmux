use crate::routing::cursor_time::later_cursor;

use super::{RoutingAction, RoutingState};

pub fn reduce(state: &RoutingState, action: &RoutingAction) -> Option<RoutingState> {
    match action {
        RoutingAction::Registered {
            session_id,
            display,
        } => {
            let mut identity = state.identity.clone();
            identity.register(session_id, display);
            Some(RoutingState { identity })
        }
        RoutingAction::TerminalLinked {
            session_id,
            terminal_id,
        } => {
            let mut identity = state.identity.clone();
            identity.link_terminal(session_id, terminal_id);
            Some(RoutingState { identity })
        }
        RoutingAction::Renamed {
            session_id,
            display,
        } => {
            let mut identity = state.identity.clone();
            identity.rename(session_id, display);
            Some(RoutingState { identity })
        }
        RoutingAction::CursorAdvanced { session_id, cursor } => {
            let current = state.identity.cursor(session_id);
            match later_cursor(current.as_deref(), Some(cursor)) {
                Some(later) if current.as_deref() != Some(later.as_str()) => {
                    let mut identity = state.identity.clone();
                    identity.advance_cursor(session_id, &later);
                    Some(RoutingState { identity })
                }
                _ => None,
            }
        }
        RoutingAction::CursorAnchored { session_id, cursor } => {
            let mut identity = state.identity.clone();
            identity.advance_cursor(session_id, cursor);
            Some(RoutingState { identity })
        }
        RoutingAction::Reaped { display } => {
            let mut identity = state.identity.clone();
            identity.reap_unclaimed_duplicates_of(display);
            Some(RoutingState { identity })
        }
    }
}
