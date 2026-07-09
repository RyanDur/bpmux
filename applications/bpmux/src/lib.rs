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
pub mod comprehension;
pub mod diag;
#[cfg(test)]
mod diag_tests;
pub mod hub;
#[cfg(test)]
mod hub_tests;
pub mod integrations;
pub mod kernel;
#[cfg(test)]
mod kernel_tests;
pub mod location;
#[cfg(test)]
mod location_tests;
pub mod log;
#[cfg(test)]
mod log_tests;
pub mod mcp;
pub mod memory;
pub mod message;
#[cfg(test)]
mod message_tests;
pub(crate) mod persist;
pub mod product_management;
pub mod rate_limit;
pub mod reactions;
#[cfg(test)]
mod reactions_tests;
pub mod room;
pub mod roster;
pub mod routing;
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
pub mod workflow;

pub use cli::bootstrap::run;
#[cfg(test)]
pub(crate) use cli::bootstrap::user_id_from_pin_or_dir;
pub(crate) use cli::bootstrap::{Cli, resolve_bpmux_port, resolve_role, resolve_user_id};
