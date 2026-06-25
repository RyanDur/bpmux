#![deny(unsafe_code)]

pub mod auth;
#[cfg(test)]
mod auth_tests;
pub mod awareness;
#[cfg(test)]
mod awareness_tests;
pub mod bookmark;
pub mod bridge;
pub mod briefing;
pub mod ci;
#[cfg(test)]
mod ci_tests;
mod cli;
pub mod comprehension;
pub mod diag;
#[cfg(test)]
mod diag_tests;
pub mod github;
pub mod integrations;
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
pub mod orchestrator;
pub mod precommit;
#[cfg(test)]
mod precommit_tests;
pub mod product_management;
pub mod reactions;
pub mod reload;
pub mod room;
pub mod routing;
pub mod server;
pub mod session;
#[cfg(test)]
mod session_id_pin_tests;
#[cfg(test)]
mod session_tests;
pub mod speech_guard;
#[cfg(test)]
mod speech_guard_tests;
pub mod storage;
#[cfg(test)]
mod storage_tests;
#[cfg(test)]
mod team_roster_tests;
pub mod telemetry;
pub mod transport;
pub mod workflow;

pub use cli::bootstrap::run;
#[cfg(test)]
pub(crate) use cli::bootstrap::session_id_from_pin_or_dir;
pub(crate) use cli::bootstrap::{Cli, resolve_bpmux_port, resolve_role, resolve_session_id};
