#![deny(unsafe_code)]

pub mod auth;
#[cfg(test)]
mod auth_tests;
pub mod bookmark;
#[cfg(test)]
mod bookmark_tests;
pub mod bridge;
pub mod briefing;
#[cfg(test)]
mod briefing_tests;
pub mod ci;
#[cfg(test)]
mod ci_tests;
mod cli;
pub use ::hub;
pub use ::integrations;
#[cfg(test)]
mod local_url_lock_tests;
pub mod location;
#[cfg(test)]
mod location_migration_tests;
#[cfg(test)]
mod location_tests;
pub use ::mcp_tools as mcp;
#[cfg(test)]
mod order_state_pin_tests;
#[cfg(test)]
mod pm_integration_tests;
pub mod room;
pub mod server;
#[cfg(test)]
mod storage_tests;
pub mod store;
pub mod sync;
#[cfg(test)]
mod team_roster_tests;
pub use ::telemetry;
#[cfg(test)]
mod tracker_contract_tests;
pub mod transport;
#[cfg(test)]
mod user_id_pin_tests;
#[cfg(test)]
mod view_integration_tests;
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
