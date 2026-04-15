//! `/share` command handler (#160) — extracted from `commands.rs` to keep
//! that file under the 400-line soft limit.
//!
//! Provides sender-explicit cross-scope propagation with a provenance
//! marker. See `#160` / `docs/turn-aware-channel-spec.md` for the design
//! rationale. Refuses to propagate messages whose `explicit_privacy` flag
//! is set — those came from `/to` and the sender's explicit privacy
//! intent overrides propagation.

use chrono::Utc;

use super::MAX_HISTORY;
use crate::session::{Message, MessageType};
use crate::terminal::app::App;

impl App {
    /// `/share <scope> [optional text]` — cross-scope propagation (#160).
    ///
    /// Two forms:
    /// - **Form 1**: `/share <scope>` with no body → propagate the most
    ///   recent message in the sender's current view to `<scope>`, with a
    ///   `[Shared by <sender> from <source-scope> thread]` provenance marker.
    /// - **Form 2**: `/share <scope> "text"` → send a fresh message to
    ///   `<scope>` with a `[Shared by <sender>]` marker (no source scope
    ///   because there's no thread to reference).
    pub(super) fn handle_share(&mut self, rest: String) -> Option<Message> {
        let rest = rest.trim();
        if rest.is_empty() {
            return self.show_share_usage();
        }
        // Parse target scope (first token) and optional body (rest).
        let (scope_str, body_opt) = match rest.split_once(' ') {
            Some((s, b)) => (s, Some(b.trim())),
            None => (rest, None),
        };
        if scope_str.is_empty() {
            return self.show_share_usage();
        }

        // Parse scope into recipient list. "all" is treated as None (full
        // broadcast), matching the existing channel/broadcast conventions.
        let target_to: Option<Vec<String>> = if scope_str == "all" {
            None
        } else {
            let recipients: Vec<String> = scope_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if recipients.is_empty() {
                return self.show_share_usage();
            }
            Some(recipients)
        };

        // Form 2: explicit text → fresh message.
        if let Some(body) = body_opt
            && !body.is_empty()
        {
            let marker = format!("[Shared by {}] {body}", self.identity.name);
            let msg = Message {
                from: self.from_identity(),
                text: marker,
                ts: Utc::now(),
                message_type: MessageType::Chat,
                to: target_to,
                addressee: None,
                explicit_privacy: false,
                multi_depth: None,
                topic: None,
            };
            self.push_message(msg.clone());
            self.clear_input();
            self.scroll.offset = 0;
            self.scroll.unread_count = 0;
            if self.input_history.len() >= MAX_HISTORY {
                self.input_history.remove(0);
            }
            self.input_history.push(self.input.clone());
            self.history_index = None;
            return Some(msg);
        }

        // Form 1: propagate the most recent in-view message.
        // Walk messages in reverse, find the newest chat message that's
        // visible in the sender's current view (matches current_view or is
        // broadcast if view is None).
        let current_view = self.current_view.clone();
        let owner = self.identity.name.clone();
        let source = self
            .messages
            .iter()
            .rev()
            .find(|m| {
                if !m.is_chat() {
                    return false;
                }
                match &current_view {
                    None => m.to.is_none(), // broadcast view sees only broadcast
                    Some(view) => crate::terminal::app::channel_name(m, &owner) == *view,
                }
            })
            .cloned();

        let Some(source) = source else {
            self.system_message("No recent message in the current view.".to_string());
            self.clear_input();
            return None;
        };

        // Privacy gate: refuse to propagate /to-scoped private messages.
        if source.explicit_privacy {
            self.system_message("Cannot share a private message.".to_string());
            self.clear_input();
            return None;
        }

        // Build the provenance marker. Source-scope is the channel name of
        // the original message (or "broadcast" if None).
        let source_scope = match &source.to {
            None => "broadcast".to_string(),
            Some(_) => crate::terminal::app::channel_name(&source, &self.identity.name),
        };
        let marker = format!(
            "[Shared by {} from {source_scope} thread] {}",
            self.identity.name, source.text
        );

        // Addressee is explicitly NOT inherited — propagated messages are
        // substrate updates (FYI), not tasks. If the original had an
        // addressee, the receiving agent treats the propagated copy as
        // awareness only.
        let msg = Message {
            from: self.from_identity(),
            text: marker,
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to: target_to,
            addressee: None,
            explicit_privacy: false,
            multi_depth: None,
            topic: None,
        };
        self.push_message(msg.clone());
        self.clear_input();
        self.scroll.offset = 0;
        self.scroll.unread_count = 0;
        Some(msg)
    }

    pub(super) fn show_share_usage(&mut self) -> Option<Message> {
        self.system_message(
            "/share <scope> [message]\n/share all — propagate last message\n/share dev \"text\" — send fresh".to_string(),
        );
        self.clear_input();
        None
    }
}
