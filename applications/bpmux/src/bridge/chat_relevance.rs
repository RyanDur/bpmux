pub(crate) const CHAT_RELEVANCE_CLASSES: [&str; 3] = ["chat", "insight", "nudge"];

pub(crate) fn chat_class_reaches_live_stream(
    message_class: &str,
    addressed_to_me: bool,
    sender_is_owner: bool,
    declared_chat_classes: &[String],
) -> bool {
    if addressed_to_me || sender_is_owner {
        return true;
    }
    if declared_chat_classes.is_empty() {
        return message_class != "nudge";
    }
    declared_chat_classes.iter().any(|c| c == message_class)
}

pub(crate) fn scope_reaches_live_stream(to: Option<&[String]>, recipient_short: &str) -> bool {
    if matches!(
        crate::transport::ws::role_from_short_name(recipient_short),
        crate::store::RosterRole::Owner
    ) {
        return true;
    }
    match to {
        None => true,
        Some(recipients) => {
            recipients
                .iter()
                .any(|r| r == crate::message::BROADCAST_ALL)
                || recipients.iter().any(|r| r == recipient_short)
        }
    }
}
