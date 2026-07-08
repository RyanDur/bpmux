mod action;
mod reduce;
mod selectors;
mod state;

#[cfg(test)]
mod reduce_tests;

pub use action::CommitmentsAction;
pub use reduce::reduce;
pub use selectors::matching_roles;
pub use state::CommitmentsState;
