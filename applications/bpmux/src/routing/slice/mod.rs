mod action;
mod hydrate;
mod reconcile;
mod reduce;
mod register_agent;
mod selectors;
mod state;

#[cfg(test)]
mod reduce_tests;
#[cfg(test)]
mod register_agent_tests;

pub use action::RoutingAction;
pub use hydrate::from_users;
pub use reconcile::{RoutingReconcile, reconcile};
pub use reduce::reduce;
pub use register_agent::register_agent_actions;
pub use selectors::{
    all_ids, cursor, delivery_targets, display_for, display_for_terminal, drain_floor,
    is_terminal_linked, missed_after_cursor, resolve, resolve_recipients, terminal_for,
    user_for_terminal,
};
pub use state::RoutingState;
