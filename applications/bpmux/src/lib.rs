#![deny(unsafe_code)]

pub use ::auth;
pub use ::bookmark;
pub use ::bridge;
pub use ::briefing;
mod cli;
pub use ::hub;
pub use ::integrations;
pub use ::location;
pub use ::mcp_tools as mcp;
pub use ::room;
pub use ::sync;
pub use ::telemetry;
pub use ::transport;

pub use cli::bootstrap::run;
pub(crate) use cli::bootstrap::{
    Cli, resolve_bpmux_port, resolve_or_register_user_id, resolve_registered_user_id, resolve_role,
};
