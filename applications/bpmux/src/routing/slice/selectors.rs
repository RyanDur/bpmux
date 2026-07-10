use crate::message::BROADCAST_ALL;
use crate::routing::cursor_time::{is_future, later_cursor};
use crate::routing::{bridge_agent_name, short_name};

use super::RoutingState;

pub fn resolve(state: &RoutingState, display: &str) -> Vec<String> {
    state.identity.resolve(display)
}

pub fn resolve_many(state: &RoutingState, displays: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    displays
        .iter()
        .flat_map(|display| resolve(state, short_name(display)))
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

pub fn drain_floor(
    state: &RoutingState,
    user_id: &str,
    client_ack: Option<&str>,
) -> Option<String> {
    let stored = state.identity.cursor(user_id);
    let trusted_ack = client_ack.filter(|ack| !is_future(ack));
    later_cursor(stored.as_deref(), trusted_ack)
}

pub fn terminal_for(state: &RoutingState, user_id: &str) -> Option<String> {
    state.identity.terminal_for(user_id)
}

pub fn display_for(state: &RoutingState, user_id: &str) -> Option<String> {
    state.identity.display_for(user_id)
}

pub fn cursor(state: &RoutingState, user_id: &str) -> Option<String> {
    state.identity.cursor(user_id)
}

pub fn user_for_terminal(state: &RoutingState, terminal_id: &str) -> Option<String> {
    state.identity.user_for_terminal(terminal_id)
}

pub fn is_terminal_linked(state: &RoutingState, terminal_id: &str) -> bool {
    state.identity.is_terminal_linked(terminal_id)
}

pub fn all_ids(state: &RoutingState) -> Vec<String> {
    state.identity.all_ids()
}

pub fn display_for_terminal(state: &RoutingState, terminal_id: &str) -> Option<String> {
    state.identity.display_for_terminal(terminal_id)
}

pub fn missed_after_cursor(
    state: &RoutingState,
    user_id: &str,
    event_cursors: &[String],
) -> Vec<String> {
    state.identity.missed_after_cursor(user_id, event_cursors)
}

pub fn resolve_recipients(state: &RoutingState, to: Option<&[String]>) -> Vec<String> {
    state.identity.resolve_recipients(to)
}

pub fn delivery_targets(
    state: &RoutingState,
    to: Option<&[String]>,
    to_ids: Option<&[String]>,
    sender: &str,
    sender_id: Option<&str>,
) -> Vec<(String, String)> {
    let identity = &state.identity;
    let sender_ids: Vec<String> = match sender_id {
        Some(id) => vec![id.to_string()],
        None => identity.resolve(short_name(sender)),
    };
    let recipient_ids: Vec<String> = match to {
        None => identity.all_ids(),
        Some(names) if names.iter().any(|name| name == BROADCAST_ALL) => identity.all_ids(),
        Some(names) => match to_ids {
            Some(ids) => ids.to_vec(),
            None => names
                .iter()
                .flat_map(|name| identity.resolve(short_name(name)))
                .collect(),
        },
    };
    recipient_ids
        .into_iter()
        .filter(|id| !sender_ids.contains(id))
        .filter_map(|id| identity.terminal_for(&id).map(|terminal| (id, terminal)))
        .filter(|(_, terminal)| bridge_agent_name(terminal) != short_name(sender))
        .collect()
}
