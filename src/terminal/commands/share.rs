//! `/share` command handler (#160) — extracted from `commands.rs` to keep
//! that file under the 400-line soft limit.
//!
//! Provides sender-explicit cross-scope propagation with a provenance
//! marker. See `#160` / `docs/turn-aware-channel-spec.md` for the design
//! rationale. Refuses to propagate messages whose `explicit_privacy` flag
//! is set — those came from `/to` and the sender's explicit privacy
//! intent overrides propagation.
//!
//! The shell method `handle_share` owns state mutation (input, scroll,
//! history); the pure helpers below own logic (scope parsing, message
//! finding, marker formatting). Extracted in #297 per the
//! functional-core-imperative-shell discipline — pure cores are unit-
//! testable without `App`, which the shell previously tangled into a
//! 120-line `&mut self` method that required the full app fixture to
//! exercise any single branch.

use chrono::Utc;

use super::MAX_HISTORY;
use crate::session::Message;
use crate::terminal::app::App;

/// Parse the scope argument of `/share <scope> [body]` into a
/// recipient list. Pure fn — no I/O, no `App` dependency.
///
/// Returns:
/// - `Some(None)` — broadcast-to-all (the `BROADCAST_ALL` sentinel)
/// - `Some(Some(recipients))` — specific recipient list, already
///   trimmed and de-duplicated of empties
/// - `None` — invalid input (empty string, or comma-list that
///   collapses to nothing after trimming). Caller's job to surface
///   usage.
pub(crate) fn parse_share_scope(scope_str: &str) -> Option<Option<Vec<String>>> {
    if scope_str.is_empty() {
        return None;
    }
    if scope_str == crate::session::message::BROADCAST_ALL {
        return Some(None);
    }
    let recipients: Vec<String> = scope_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if recipients.is_empty() {
        None
    } else {
        Some(Some(recipients))
    }
}

/// Find the newest chat message visible in the given view. Pure fn
/// over an owned message slice.
///
/// View semantics:
/// - `None` (broadcast view) → only broadcast messages (`to.is_none()`)
///   match, not direct-to-X messages
/// - `Some(view)` → messages whose resolved channel name equals `view`
///   match (lets /share propagate from a direct channel view back
///   to broadcast, etc.)
///
/// `owner` is the viewer's own identity name — required because
/// `channel_name` resolves the message's channel from the viewer's
/// perspective (a direct message from `alice` in `bob`'s session
/// lives under channel `alice`, not `bob`).
pub(crate) fn find_shareable_message<'a>(
    messages: &'a [Message],
    current_view: &Option<String>,
    owner: &str,
) -> Option<&'a Message> {
    messages.iter().rev().find(|m| {
        if !m.is_chat() {
            return false;
        }
        match current_view {
            None => m.to.is_none(),
            Some(view) => crate::terminal::app::channel_name(m, owner) == *view,
        }
    })
}

/// Resolve the source scope for a Form 1 propagation marker.
/// `None` recipients → `"broadcast"` sentinel; otherwise the
/// viewer-perspective channel name. Pure fn over owned data.
pub(crate) fn build_source_scope(source: &Message, owner: &str) -> String {
    match &source.to {
        None => "broadcast".to_string(),
        Some(_) => crate::terminal::app::channel_name(source, owner),
    }
}

/// Build the provenance marker string for a shared message.
///
/// - `source_scope: Some(scope)` → Form 1: existing-message
///   propagation. Output: `"[Shared by {actor} from {scope} thread] {text}"`
/// - `source_scope: None` → Form 2: fresh-body share. Output:
///   `"[Shared by {actor}] {text}"` (no source-scope because there's
///   no thread to reference — the body was typed at /share time)
pub(crate) fn build_share_marker(actor: &str, source_scope: Option<&str>, text: &str) -> String {
    match source_scope {
        None => format!("[Shared by {actor}] {text}"),
        Some(scope) => format!("[Shared by {actor} from {scope} thread] {text}"),
    }
}

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
    pub(in crate::terminal) fn handle_share(&mut self, rest: String) -> Option<Message> {
        let rest = rest.trim();
        if rest.is_empty() {
            return self.show_share_usage();
        }
        // Parse target scope (first token) and optional body (rest).
        let (scope_str, body_opt) = match rest.split_once(' ') {
            Some((s, b)) => (s, Some(b.trim())),
            None => (rest, None),
        };

        let target_to = match parse_share_scope(scope_str) {
            Some(recipients) => recipients,
            None => return self.show_share_usage(),
        };

        // Form 2: explicit text → fresh message.
        if let Some(body) = body_opt
            && !body.is_empty()
        {
            let marker = build_share_marker(&self.identity.name, None, body);
            let msg = Message {
                from: self.from_identity(),
                text: marker,
                ts: Utc::now(),
                to: target_to,
                ..Message::default()
            };
            self.push_message(msg.clone());
            self.clear_input();
            self.scroll.offset = 0;
            self.scroll.unread_count = 0;
            if self.editor.input_history.len() >= MAX_HISTORY {
                self.editor.input_history.remove(0);
            }
            self.editor.input_history.push(self.editor.input.clone());
            self.editor.history_index = None;
            return Some(msg);
        }

        // Form 1: propagate the most recent in-view message.
        let current_view = self.channels.current_view.clone();
        let owner = self.identity.name.clone();
        let source = find_shareable_message(&self.msg.messages, &current_view, &owner).cloned();

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

        let source_scope = build_source_scope(&source, &self.identity.name);
        let marker = build_share_marker(&self.identity.name, Some(&source_scope), &source.text);

        // Addressee is explicitly NOT inherited — propagated messages are
        // substrate updates (FYI), not tasks. If the original had an
        // addressee, the receiving agent treats the propagated copy as
        // awareness only.
        let msg = Message {
            from: self.from_identity(),
            text: marker,
            ts: Utc::now(),
            to: target_to,
            ..Message::default()
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
