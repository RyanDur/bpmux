pub const CHAT_RELEVANCE_CLASSES: [&str; 3] = ["chat", "insight", "nudge"];

pub fn message_type_reaches_llm_stream(message_type: Option<&str>) -> bool {
    !matches!(message_type, Some("activity") | Some("system"))
}

pub fn chat_class_reaches_live_stream(
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

pub fn chat_event_classes(channel: &str, addressee: Option<&str>) -> Vec<&'static str> {
    let mut classes = Vec::new();
    if channel == "broadcast" {
        classes.push("broadcast_received");
    }
    if matches!(addressee, Some(a) if a != message::BROADCAST_ALL) {
        classes.push("addressed_to_me");
    }
    classes
}

pub fn scope_reaches_live_stream(
    to: Option<&[String]>,
    to_ids: Option<&[String]>,
    user_id: Option<&str>,
    recipient_short: &str,
) -> bool {
    if matches!(
        roster::roster_role_for_identity_prefix(recipient_short),
        store::RosterRole::Owner
    ) {
        return true;
    }
    message::to_reaches(to, to_ids, user_id, recipient_short)
}

pub fn addressed_to_me(
    addressee: Option<&str>,
    addressee_ids: Option<&[String]>,
    to: Option<&[String]>,
    to_ids: Option<&[String]>,
    user_id: Option<&str>,
    display: &str,
) -> bool {
    message::addressee_matches(addressee, addressee_ids, user_id, display)
        || named_directly(to, to_ids, user_id, display)
}

fn named_directly(
    to: Option<&[String]>,
    to_ids: Option<&[String]>,
    user_id: Option<&str>,
    display: &str,
) -> bool {
    if to.is_none_or(|names| names.iter().any(|r| r == message::BROADCAST_ALL)) {
        return false;
    }
    message::to_reaches(to, to_ids, user_id, display)
}
