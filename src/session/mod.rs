pub mod log;
pub mod message;

pub use log::{
    LogError, append_message, broadcast_message, create_session, default_sessions_base,
    read_messages, resume_or_create_session,
};
pub use message::Message;

pub const ROLE_HUMAN: &str = "human";
pub const ROLE_MCP_CLIENT: &str = "mcp-client";
pub const ROLE_WS: &str = "ws";

pub fn participant_id(name: &str, role: &str) -> String {
    format!("{name}:{role}")
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn builds_participant_id_from_name_and_role() {
        assert_eq!(participant_id("ryan", ROLE_HUMAN), "ryan:human");
        assert_eq!(participant_id("dev", ROLE_MCP_CLIENT), "dev:mcp-client");
        assert_eq!(participant_id("claude-code", ROLE_WS), "claude-code:ws");
    }
}
