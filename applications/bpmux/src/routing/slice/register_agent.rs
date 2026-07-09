use crate::routing::cursor_time::cursor_after;

use super::{RoutingAction, RoutingState};

pub fn register_agent_actions(
    state: &RoutingState,
    session_id: &str,
    display: &str,
    terminal_id: &str,
    job_start: &str,
) -> Vec<RoutingAction> {
    let stored = state.identity.cursor(session_id);
    let fresh = stored.is_none();
    let anchored = fresh || cursor_after(stored.as_deref(), job_start);
    let mut actions = Vec::new();
    if fresh {
        actions.push(RoutingAction::Registered {
            session_id: session_id.to_string(),
            display: display.to_string(),
        });
    }
    if anchored {
        actions.push(RoutingAction::CursorAnchored {
            session_id: session_id.to_string(),
            cursor: job_start.to_string(),
        });
    }
    actions.push(RoutingAction::TerminalLinked {
        session_id: session_id.to_string(),
        terminal_id: terminal_id.to_string(),
    });
    actions.push(RoutingAction::Reaped {
        display: display.to_string(),
    });
    actions
}
