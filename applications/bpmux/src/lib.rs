#![deny(unsafe_code)]

pub mod board;
pub mod bridge;
pub mod ci;
#[cfg(test)]
mod ci_tests;
mod cli;
pub mod diag;
#[cfg(test)]
mod diag_tests;
pub mod github;
pub mod mcp;
pub mod orchestrator;
pub mod reload;
pub mod roadmap;
pub mod room;
pub mod server;
pub mod session;
#[cfg(test)]
mod session_id_pin_tests;
pub mod transport;
pub mod workflow;

pub use cli::bootstrap::run;
#[cfg(test)]
pub(crate) use cli::bootstrap::session_id_from_pin_or_dir;
pub(crate) use cli::bootstrap::{Cli, resolve_bpmux_port, resolve_role, resolve_session_id};
