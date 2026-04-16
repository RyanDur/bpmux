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
