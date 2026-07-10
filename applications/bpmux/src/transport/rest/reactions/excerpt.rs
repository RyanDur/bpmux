use super::ReactionToggle;

pub(crate) const EXCERPT_MAX_LEN: usize = 40;

pub(crate) fn truncate_excerpt(text: &str, max_len: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let flat = flat.trim();
    if flat.chars().count() <= max_len {
        return flat.to_string();
    }
    let mut last_space: Option<usize> = None;
    let mut cut: usize = flat.len();
    for (i, (byte_idx, ch)) in flat.char_indices().enumerate() {
        if i == max_len {
            cut = byte_idx;
            break;
        }
        if ch.is_whitespace() {
            last_space = Some(byte_idx);
        }
    }
    let snip_end = last_space.unwrap_or(cut);
    let mut out = flat[..snip_end].trim_end().to_string();
    out.push('…');
    out
}

pub(crate) fn find_message_excerpt(
    messages: &[message::Message],
    message_key: &str,
    max_len: usize,
) -> Option<String> {
    let (ts_str, from) = message_key.split_once('|')?;
    if ts_str.is_empty() || from.is_empty() {
        return None;
    }
    for m in messages {
        if m.from != from {
            continue;
        }
        if m.ts.to_rfc3339() != ts_str {
            continue;
        }
        if m.text.is_empty() {
            return None;
        }
        return Some(truncate_excerpt(&m.text, max_len));
    }
    None
}

pub(crate) fn message_scope_from_key(
    messages: &[message::Message],
    message_key: &str,
) -> Option<Vec<String>> {
    let (ts_str, from) = message_key.split_once('|')?;
    if ts_str.is_empty() || from.is_empty() {
        return None;
    }
    for m in messages {
        if m.from != from || m.ts.to_rfc3339() != ts_str {
            continue;
        }
        let recipients = m.to.as_ref()?;
        if recipients.is_empty() || recipients.iter().any(|r| r == message::BROADCAST_ALL) {
            return None;
        }
        let sender_short = from.split(':').next().unwrap_or(from);
        let mut scope: Vec<String> = recipients.clone();
        if !scope.iter().any(|p| p == sender_short) {
            scope.push(sender_short.to_string());
        }
        return Some(scope);
    }
    None
}

pub(crate) fn build_reaction_event_text_with_excerpt(
    body: &ReactionToggle,
    present: bool,
    excerpt: Option<&str>,
) -> String {
    let original_sender_short = body
        .message_key
        .split_once('|')
        .map(|(_ts, from)| from.split(':').next().unwrap_or(from).to_string());
    let base = match (present, original_sender_short) {
        (true, Some(sender)) if !sender.is_empty() => {
            format!(
                "{} reacted {} to {}'s message",
                body.user, body.emoji, sender
            )
        }
        (true, _) => format!("{} reacted {}", body.user, body.emoji),
        (false, Some(sender)) if !sender.is_empty() => {
            format!(
                "{} unreacted {} from {}'s message",
                body.user, body.emoji, sender
            )
        }
        (false, _) => format!("{} unreacted {}", body.user, body.emoji),
    };
    match excerpt {
        Some(e) if !e.is_empty() => format!("{base}: “{e}”"),
        _ => base,
    }
}
