use message::MessageType;

pub(crate) const MAX_MESSAGE_SIZE: usize = 65536;

pub(crate) fn should_deliver(
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

pub(crate) fn message_size_ok(text: &str) -> bool {
    text.len() <= MAX_MESSAGE_SIZE
}
