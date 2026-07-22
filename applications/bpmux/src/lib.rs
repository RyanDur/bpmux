#![deny(unsafe_code)]

pub use ::auth;
pub use ::bookmark;
pub use ::bridge;
pub use ::briefing;
pub mod ci;
#[cfg(test)]
mod ci_tests;
mod cli;
pub use ::hub;
pub use ::integrations;
#[cfg(test)]
mod local_url_lock_tests;
pub use ::location;
pub use ::mcp_tools as mcp;
#[cfg(test)]
mod order_state_pin_tests;
#[cfg(test)]
mod pm_integration_tests;
pub use ::room;
pub mod server;
#[cfg(test)]
mod storage_tests;
pub mod store;
pub use ::sync;
#[cfg(test)]
mod sync_drain_listener_tests;
#[cfg(test)]
mod team_roster_tests;
#[cfg(test)]
mod transport_excerpt_enrichment_tests;
#[cfg(test)]
mod transport_presence_endpoint_tests;
#[cfg(test)]
mod transport_ws_client_handlers_tests;
#[cfg(test)]
mod transport_ws_roster_projection_tests;
pub use ::telemetry;
#[cfg(test)]
mod tracker_contract_tests;
pub use ::transport;
#[cfg(test)]
mod view_integration_tests;

pub use cli::bootstrap::run;
pub(crate) use cli::bootstrap::{
    Cli, resolve_bpmux_port, resolve_or_register_user_id, resolve_registered_user_id, resolve_role,
};
