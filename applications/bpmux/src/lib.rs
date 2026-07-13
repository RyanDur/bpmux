#![deny(unsafe_code)]

pub mod auth;
#[cfg(test)]
mod auth_tests;
pub mod awareness;
#[cfg(test)]
mod awareness_tests;
pub mod board;
pub mod bookmark;
#[cfg(test)]
mod bookmark_tests;
pub mod bridge;
pub mod briefing;
#[cfg(test)]
mod briefing_tests;
pub mod ci;
pub(crate) mod ci_link;
#[cfg(test)]
mod ci_tests;
mod cli;
pub mod hub;
#[cfg(test)]
mod hub_tests;
pub mod integrations;
pub mod location;
#[cfg(test)]
mod location_migration_tests;
#[cfg(test)]
mod location_tests;
pub mod mcp;
#[cfg(test)]
mod order_state_pin_tests;
pub mod owner_session;
#[cfg(test)]
mod pm_integration_tests;
pub mod rate_limit;
pub mod reactions;
#[cfg(test)]
mod reactions_tests;
pub mod room;
pub mod roster;
pub mod server;
pub mod storage;
#[cfg(test)]
mod storage_tests;
pub mod store;
pub mod sync;
#[cfg(test)]
mod team_roster_tests;
pub mod telemetry;
pub mod transport;
#[cfg(test)]
mod user_id_pin_tests;
pub mod view;
#[cfg(test)]
mod workflow_test_support;

pub use cli::bootstrap::run;
pub(crate) use cli::bootstrap::{
    Cli, resolve_bpmux_port, resolve_or_register_user_id, resolve_registered_user_id, resolve_role,
};
#[cfg(test)]
pub(crate) use cli::bootstrap::{
    or_register_user_id_from_pin_or_dir, registered_user_id_from_pin_or_dir,
};
