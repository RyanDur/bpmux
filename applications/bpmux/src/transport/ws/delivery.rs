use super::role_from_short_name;
use crate::message::MessageType;

pub(crate) const MAX_MESSAGE_SIZE: usize = 65536;

pub(crate) fn short_name(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub(crate) fn should_deliver(msg: &crate::message::Message, peer_identity: &str) -> bool {
    if matches!(
        role_from_short_name(short_name(peer_identity)),
        crate::store::stores::RosterRole::Owner
    ) {
        return msg.message_type != MessageType::Nudge;
    }
    let recipients = match &msg.to {
        None => return true,
        Some(r) => r,
    };
    if recipients
        .iter()
        .any(|r| r == crate::message::BROADCAST_ALL)
    {
        return true;
    }
    let peer_short = short_name(peer_identity);
    let sender_short = short_name(&msg.from);
    if peer_short == sender_short {
        return true;
    }
    recipients.iter().any(|r| short_name(r) == peer_short)
}

pub(crate) fn message_size_ok(text: &str) -> bool {
    text.len() <= MAX_MESSAGE_SIZE
}
