//! Per-rule predicates for the chat-pane filter index (#262).
//!
//! `App::rebuild_filtered_if_dirty` was a 50-line method that touched
//! 6 sub-states and mixed pipeline rules with mutation. The rules are
//! now pure functions taking `&FilterContext` + `&Message`, and the
//! index-building loop becomes a thin map over a single classifier.
//!
//! The context is built field-by-field at the call site (rather than
//! `from_app`) so the borrow checker sees each input as a disjoint
//! sub-state borrow — leaving `&mut self.msg` free for the loop body
//! to write filtered_indices / cached_hidden_count.

use crate::session::Message;

use super::channel_name;

/// Read-only inputs the filter rules need. Built per call to
/// `rebuild_filtered_if_dirty`; cheap (refs into App, no clones).
pub(super) struct FilterContext<'a> {
    pub show_system_messages: bool,
    pub show_all_classes: bool,
    pub current_view: Option<&'a str>,
    pub identity_name: &'a str,
    pub focused_roles: Option<&'a [String]>,
    /// Active thread id (#268). When `Some`, the viewer is inside a
    /// thread and the chat pane scopes to messages carrying that
    /// thread_id. When `None`, threads behave as broadcast (all
    /// messages visible, thread-tagged ones render their thread
    /// name alongside).
    pub current_thread: Option<&'a str>,
    /// CI-events-only filter (#295). When true, every non-CI message
    /// Skip-s and only CI-transition broadcasts Include. Intentionally
    /// overrides thread view, current_view, class filtering, focus —
    /// the purpose is "find CI events in a long scrollback" + the
    /// other filters would defeat that. Toggled via the CI events
    /// sidebar section; state persists in `UiState::ci_filter_active`.
    pub ci_filter_active: bool,
}

/// Three-way verdict so the caller can distinguish "this message
/// should be visible" from "skipped silently" from "hidden but
/// counted in the cached_hidden_count for the status bar."
pub(super) enum FilterVerdict {
    Include,
    Skip,
    Hidden,
}

/// Classify one message against the filter pipeline. Pure function
/// over (ctx, msg) — testable without an App.
pub(super) fn classify(ctx: &FilterContext, m: &Message) -> FilterVerdict {
    // CI-events-only filter (#295). When active, it OVERRIDES every
    // other filter: thread scope, current_view, class filter, focus.
    // Reasoning: the user turned this on to find CI events across
    // the whole log; applying thread/view on top would defeat that.
    // Only `Activity` messages with the `🔴 CI` / `🟢 CI` prefix
    // pass through; everything else Skips.
    if ctx.ci_filter_active {
        return if crate::terminal::ci::is_ci_event_message(m) {
            FilterVerdict::Include
        } else {
            FilterVerdict::Skip
        };
    }
    // Thread-tagged messages render in their own thread view
    // regardless of message type — scope-entry markers, intra-thread
    // nudges, etc. are thread-local by construction and belong in
    // the thread's log (#268 re-work: "— entered thread: X —" marker
    // was invisible before this because it was System-typed).
    if let Some(current) = ctx.current_thread {
        return match m.thread_id.as_deref() {
            Some(tid) if tid == current => FilterVerdict::Include,
            _ => FilterVerdict::Skip,
        };
    }
    if !m.is_chat() {
        return if ctx.show_system_messages {
            FilterVerdict::Include
        } else {
            FilterVerdict::Skip
        };
    }
    if !in_channel(ctx, m) {
        return FilterVerdict::Skip;
    }
    if !ctx.show_all_classes && crate::terminal::chat::should_hide_by_class(m, ctx.identity_name) {
        return FilterVerdict::Hidden;
    }
    if !crate::terminal::commands::focus::in_focus(m, ctx.focused_roles, ctx.identity_name) {
        return FilterVerdict::Skip;
    }
    FilterVerdict::Include
}

/// Per-view inclusion test. Mirrors the match arms inline in the
/// previous god method.
fn in_channel(ctx: &FilterContext, m: &Message) -> bool {
    match ctx.current_view {
        None => true,
        Some("broadcast") => channel_name(m, ctx.identity_name) == "broadcast",
        Some(view) => {
            crate::terminal::markdown::message_matches_channel(m, view, ctx.identity_name)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn ctx_in_thread(tid: &'static str) -> FilterContext<'static> {
        FilterContext {
            show_system_messages: false,
            show_all_classes: true,
            current_view: None,
            identity_name: "ryan",
            focused_roles: None,
            current_thread: Some(tid),
            ci_filter_active: false,
        }
    }

    fn ctx_broadcast() -> FilterContext<'static> {
        FilterContext {
            show_system_messages: false,
            show_all_classes: true,
            current_view: None,
            identity_name: "ryan",
            focused_roles: None,
            current_thread: None,
            ci_filter_active: false,
        }
    }

    fn chat_msg(thread_id: Option<&str>) -> Message {
        Message {
            from: "designer:cc".to_string(),
            text: "hi".to_string(),
            ts: Utc::now(),
            thread_id: thread_id.map(String::from),
            ..Message::default()
        }
    }

    #[test]
    fn thread_view_includes_matching_thread_id() {
        let ctx = ctx_in_thread("cheese-123");
        let m = chat_msg(Some("cheese-123"));
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Include));
    }

    #[test]
    fn thread_view_skips_broadcast_messages() {
        let ctx = ctx_in_thread("cheese-123");
        let m = chat_msg(None);
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Skip));
    }

    #[test]
    fn thread_view_skips_other_thread_messages() {
        let ctx = ctx_in_thread("cheese-123");
        let m = chat_msg(Some("wine-456"));
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Skip));
    }

    #[test]
    fn broadcast_view_includes_thread_messages() {
        let ctx = ctx_broadcast();
        let m = chat_msg(Some("cheese-123"));
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Include));
    }

    #[test]
    fn thread_view_overrides_focus() {
        let roles = vec!["pm".to_string()];
        let ctx = FilterContext {
            focused_roles: Some(&roles),
            ..ctx_in_thread("cheese-123")
        };
        // Message from designer (not in focused_roles) but in current
        // thread — thread wins, message renders.
        let m = chat_msg(Some("cheese-123"));
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Include));
    }

    // #295 CI-events-only filter — exercises the early-return override
    // that makes ci_filter_active hide everything except
    // Activity-with-CI-prefix. Thread/view/focus all get bypassed
    // intentionally; the filter's job is "find CI events anywhere."

    fn ci_event(prefix: &str) -> Message {
        Message {
            from: "bpmux".to_string(),
            text: format!("{prefix} CI failed — abc123"),
            ts: Utc::now(),
            message_type: crate::session::MessageType::Activity,
            ..Message::default()
        }
    }

    fn ctx_ci_filter_active() -> FilterContext<'static> {
        FilterContext {
            ci_filter_active: true,
            ..ctx_broadcast()
        }
    }

    #[test]
    fn ci_filter_includes_failure_prefix() {
        let ctx = ctx_ci_filter_active();
        let m = ci_event("\u{1F534}");
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Include));
    }

    #[test]
    fn ci_filter_includes_recovery_prefix() {
        let ctx = ctx_ci_filter_active();
        let m = ci_event("\u{1F7E2}");
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Include));
    }

    #[test]
    fn ci_filter_skips_chat_messages() {
        // Even from bpmux — only Activity-with-CI-prefix passes.
        // Regular chat broadcasts would drown out the CI signal
        // the user turned on the filter to isolate.
        let ctx = ctx_ci_filter_active();
        let m = chat_msg(None);
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Skip));
    }

    #[test]
    fn ci_filter_skips_activity_without_ci_prefix() {
        // Other Activity messages (thread lifecycle, status-bar
        // broadcasts, non-CI system events) also hide during the
        // filter — the prefix check keeps the scope tight.
        let ctx = ctx_ci_filter_active();
        let m = Message {
            from: "bpmux".to_string(),
            text: "\u{1F7E0} merge conflict detected".to_string(),
            ts: Utc::now(),
            message_type: crate::session::MessageType::Activity,
            ..Message::default()
        };
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Skip));
    }

    #[test]
    fn ci_filter_overrides_thread_view() {
        // Viewer is inside a thread. Without the filter, only
        // thread-tagged messages render. With the filter active,
        // a CI event from outside the thread renders anyway — the
        // filter's "find CI events" intent overrides the thread
        // scope.
        let ctx = FilterContext {
            ci_filter_active: true,
            ..ctx_in_thread("cheese-123")
        };
        let m = ci_event("\u{1F534}");
        // CI event has no thread_id — without override it'd Skip.
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Include));
    }

    #[test]
    fn ci_filter_inactive_does_not_disturb_default_path() {
        // Default ci_filter_active=false — regular classification
        // applies. Regression guard against "forgot to gate the
        // early-return on ci_filter_active" style bugs.
        let ctx = ctx_broadcast();
        let m = ci_event("\u{1F534}");
        // Activity messages aren't chat → show_system_messages=false
        // in this ctx → FilterVerdict::Skip, matching pre-#295 shape.
        assert!(matches!(classify(&ctx, &m), FilterVerdict::Skip));
    }
}
