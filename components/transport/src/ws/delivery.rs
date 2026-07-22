use message::MessageType;

pub const MAX_MESSAGE_SIZE: usize = 65536;

pub fn should_deliver(
    msg: &message::Message,
    user_id: Option<&str>,
    peer_identity: &str,
    is_owner: bool,
) -> bool {
    if is_owner {
        return msg.message_type != MessageType::Nudge;
    }
    msg.is_from_user(user_id, peer_identity) || msg.reaches_user(user_id, peer_identity)
}

pub fn message_size_ok(text: &str) -> bool {
    text.len() <= MAX_MESSAGE_SIZE
}
