#![deny(unsafe_code)]

pub mod auth;
#[cfg(test)]
mod auth_tests;
pub mod board;
pub mod bridge;
pub mod ci;
#[cfg(test)]
mod ci_tests;
mod cli;
pub mod comprehension;
pub mod diag;
#[cfg(test)]
mod diag_tests;
pub mod github;
pub mod mcp;
pub mod memory;
pub mod message;
#[cfg(test)]
mod message_tests;
pub mod orchestrator;
pub mod reload;
pub mod roadmap;
pub mod room;
pub mod routing;
pub mod server;
pub mod session;
#[cfg(test)]
mod session_id_pin_tests;
pub mod telemetry;
pub mod transport;
pub mod velocity;
#[cfg(test)]
mod velocity_tests;
pub mod workflow;

pub use cli::bootstrap::run;
#[cfg(test)]
pub(crate) use cli::bootstrap::session_id_from_pin_or_dir;
pub(crate) use cli::bootstrap::{Cli, resolve_bpmux_port, resolve_role, resolve_session_id};
