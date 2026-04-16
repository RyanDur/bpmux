use super::MAX_HISTORY;
use crate::session::{Message, MessageType};
use crate::terminal::app::App;
use chrono::Utc;

impl App {
    pub(super) fn send_chat_message(&mut self, text: String) -> Option<Message> {
        // Parse `@role ` prefix as an attention signal (#158).
        // `@pm what about X?` sets addressee="pm" and strips the prefix.
        // Distinct from `/to pm "..."` which sets the privacy filter — the
        // `@role` prefix is advisory addressing on broadcast messages.
        let (addressee, text) = parse_addressee_prefix(&text);
        // Derive `to` from send_target if set (sidebar selection)
        let to = self.channels.send_target.as_ref().and_then(|ch| {
            if ch == "broadcast" {
                return None; // broadcast = no `to`
            }
            // Parse channel name back to recipient list, excluding self
            let names: Vec<String> = if ch.contains('\u{2194}') {
                ch.split('\u{2194}').map(String::from).collect()
            } else if ch.contains('+') {
                ch.split('+').map(String::from).collect()
            } else {
                vec![ch.clone()]
            };
            let recipients: Vec<String> = names
                .into_iter()
                .filter(|n| n != &self.identity.name)
                .collect();
            if recipients.is_empty() {
                None
            } else {
                Some(recipients)
            }
        });
        let msg = Message {
            from: self.from_identity(),
            text,
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to,
            addressee,
            explicit_privacy: false,
            multi_depth: None,
            topic: if self.topic.active == "general" {
                None
            } else {
                Some(self.topic.active.clone())
            },
        };
        self.push_message(msg.clone());
        self.clear_input();
        self.scroll.offset = 0;
        self.scroll.unread_count = 0;
        if self.input_history.len() >= MAX_HISTORY {
            self.input_history.remove(0);
        }
        self.input_history.push(msg.text.clone());
        self.history_index = None;
        Some(msg)
    }
}

/// Parse an optional `@role ` prefix from chat input (#158). Returns
/// `(addressee, remaining_text)`. Only matches a leading `@` followed by
/// alphanumeric/hyphen characters and a single space — conservative on
/// purpose so `foo @mention bar` in the body doesn't accidentally set an
/// addressee. Returns `(None, original_text)` if no prefix is present.
///
/// Accepts `@all` as an explicit broadcast addressee (sets
/// addressee="all") because the spec lists "all" alongside role names.
pub(super) fn parse_addressee_prefix(text: &str) -> (Option<String>, String) {
    if !text.starts_with('@') {
        return (None, text.to_string());
    }
    // Walk past '@' and collect alphanumeric/hyphen chars until whitespace.
    let mut chars = text.char_indices();
    chars.next(); // consume '@'
    let mut end = 1;
    for (i, c) in chars {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            end = i + c.len_utf8();
            continue;
        }
        break;
    }
    // Must have at least one char after '@' AND be followed by whitespace
    // to count as a prefix. Bare `@` or `@role` with no body is passthrough.
    let role = &text[1..end];
    if role.is_empty() {
        return (None, text.to_string());
    }
    let rest = &text[end..];
    let rest_trimmed = rest.trim_start();
    if rest.len() == rest_trimmed.len() {
        // No whitespace after `@role` — treat as normal text, not a prefix.
        return (None, text.to_string());
    }
    (Some(role.to_string()), rest_trimmed.to_string())
}
