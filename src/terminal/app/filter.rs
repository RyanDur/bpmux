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
    pub active_topic: &'a str,
    /// Active thread id (#268). When `Some`, the viewer is inside a
    /// thread and the chat pane scopes to messages carrying that
    /// thread_id. When `None`, threads behave as broadcast (all
    /// messages visible, thread-tagged ones render their thread
    /// name alongside).
    pub current_thread: Option<&'a str>,
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
    if !m.is_chat() {
        return if ctx.show_system_messages {
            FilterVerdict::Include
        } else {
            FilterVerdict::Skip
        };
    }
    // Thread scope wins when the viewer is in a thread (#268). Only
    // messages matching the current thread render; focus stays off
    // per spec ("focus does NOT apply inside threads"). Cross-thread
    // and broadcast messages skip (the peripheral counter picks
    // them up elsewhere).
    if let Some(current) = ctx.current_thread {
        return match m.thread_id.as_deref() {
            Some(tid) if tid == current => FilterVerdict::Include,
            _ => FilterVerdict::Skip,
        };
    }
    if !in_channel(ctx, m) {
        return FilterVerdict::Skip;
    }
    if !ctx.show_all_classes
        && crate::terminal::chat::should_hide_by_class(m, ctx.identity_name)
    {
        return FilterVerdict::Hidden;
    }
    if !crate::terminal::commands::focus::in_focus(m, ctx.focused_roles, ctx.identity_name) {
        return FilterVerdict::Skip;
    }
    if let Some(msg_topic) = &m.topic
        && msg_topic != ctx.active_topic
    {
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
        Some(view) => crate::terminal::markdown::message_matches_channel(m, view, ctx.identity_name),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::session::MessageType;
    use chrono::Utc;

    fn ctx_in_thread(tid: &'static str) -> FilterContext<'static> {
        FilterContext {
            show_system_messages: false,
            show_all_classes: true,
            current_view: None,
            identity_name: "ryan",
            focused_roles: None,
            active_topic: "general",
            current_thread: Some(tid),
        }
    }

    fn ctx_broadcast() -> FilterContext<'static> {
        FilterContext {
            show_system_messages: false,
            show_all_classes: true,
            current_view: None,
            identity_name: "ryan",
            focused_roles: None,
            active_topic: "general",
            current_thread: None,
        }
    }

    fn chat_msg(thread_id: Option<&str>) -> Message {
        Message {
            from: "designer:cc".to_string(),
            text: "hi".to_string(),
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to: None,
            addressee: None,
            explicit_privacy: false,
            multi_depth: None,
            topic: None,
            thread_id: thread_id.map(String::from),
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
}
