use super::chat::BROADCAST_ALL;

fn short_name(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub fn to_reaches(
    to: Option<&[String]>,
    to_ids: Option<&[String]>,
    user_id: Option<&str>,
    display: &str,
) -> bool {
    let display = short_name(display);
    match to {
        None => return true,
        Some(recipients) if recipients.iter().any(|r| r == BROADCAST_ALL) => return true,
        _ => {}
    }
    match to_ids {
        Some(ids) => user_id.is_some_and(|id| ids.iter().any(|i| i == id)),
        None => to.is_some_and(|names| names.iter().any(|r| short_name(r) == display)),
    }
}

pub fn from_matches(
    from: &str,
    sender_id: Option<&str>,
    user_id: Option<&str>,
    display: &str,
) -> bool {
    match (sender_id, user_id) {
        (Some(sender_id), Some(caller_id)) => sender_id == caller_id,
        _ => short_name(from) == short_name(display),
    }
}

pub fn addressee_matches(
    addressee: Option<&str>,
    addressee_ids: Option<&[String]>,
    user_id: Option<&str>,
    display: &str,
) -> bool {
    let display = short_name(display);
    match addressee_ids {
        Some(ids) => user_id.is_some_and(|id| ids.iter().any(|i| i == id)),
        None => match addressee {
            None => false,
            Some(target) if target == BROADCAST_ALL => false,
            Some(target) => short_name(target) == display,
        },
    }
}
