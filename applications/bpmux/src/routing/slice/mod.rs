mod action;
mod hydrate;
mod reduce;
mod selectors;
mod state;

#[cfg(test)]
mod reduce_tests;

pub use action::RoutingAction;
pub use hydrate::from_users;
pub use reduce::reduce;
pub use selectors::{
    all_ids, cursor, delivery_targets, display_for, display_for_terminal, is_terminal_linked,
    missed_after_cursor, resolve, resolve_recipients, session_for_terminal, terminal_for,
};
pub use state::RoutingState;
