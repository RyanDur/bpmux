use crate::message::MessageType;
use crate::roster::roster_role_for_identity_prefix;
use crate::routing::short_name;

pub(crate) const MAX_MESSAGE_SIZE: usize = 65536;

pub(crate) fn should_deliver(
    msg: &crate::message::Message,
    user_id: Option<&str>,
    peer_identity: &str,
) -> bool {
    if matches!(
        roster_role_for_identity_prefix(short_name(peer_identity)),
        crate::store::RosterRole::Owner
    ) {
        return msg.message_type != MessageType::Nudge;
    }
    msg.is_from_user(user_id, peer_identity) || msg.reaches_user(user_id, peer_identity)
}

pub(crate) fn message_size_ok(text: &str) -> bool {
    text.len() <= MAX_MESSAGE_SIZE
}
