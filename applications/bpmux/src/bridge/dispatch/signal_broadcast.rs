use std::sync::Arc;

pub(super) fn broadcast_client_msg_via_hub(
    hub: &Arc<crate::hub::Hub>,
    agent_name: &str,
    msg: protocol::ClientMessage,
) {
    if let protocol::ClientMessage::Send {
        text,
        to,
        addressee,
        message_type,
    } = msg
    {
        let explicit_privacy = to.is_some();
        let from_short = user_routing::short_name(agent_name);
        let hub_msg = message::Message {
            from: format!("{from_short}:claude-code"),
            text,
            ts: chrono::Utc::now(),
            message_type: message::MessageType::from(message_type.as_deref()),
            to,
            addressee,
            explicit_privacy,
            ..message::Message::default()
        };
        if let Err(e) = hub.broadcast(&hub_msg) {
            diag::log_diag(&format!("rpc: workflow-signal broadcast failed: {e}"));
        }
    }
}
