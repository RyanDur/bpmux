//! Chat area rendering — message filtering, truncation, and display.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::session;
use crate::terminal::app::{self, App, TRUNCATE_AFTER_LINES};
use crate::terminal::markdown::{
    message_matches_channel, render_md_message, wrap_line_with_indent,
};
use crate::terminal::theme::{
    DRACULA_BG, DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_PURPLE, DRACULA_SELECTION,
    channel_tag_color, sender_color,
};

/// Class-filter predicate for #161 / #186. Returns `true` when the
/// message should be hidden in the default view (when
/// `show_all_classes` is OFF). Hidden messages reappear when the
/// user toggles `/all`.
///
/// Rules:
/// - **Typed `Nudge` messages** (classifier-authored experience
///   nudges, handoff liveness warnings, awareness signals) are
///   always hidden. This is #186's load-bearing change — before
///   it, the filter relied on a `📝` text prefix, so any nudge
///   that forgot the marker (like the handoff warning) slipped
///   through.
/// - **Legacy `📝`-prefixed bpmux-sender messages** are still
///   hidden so historical session logs written before the
///   `Nudge` type existed continue to filter correctly.
/// - **Non-bpmux messages classified as `Routine`** are hidden —
///   the existing #161 behavior for peer-sent routine content.
/// - **Everything else** (tool output like `/board`, `/roadmap`,
///   `/who`, `/debug`; regular chat; system notices) is shown.
///   Critical: we do NOT run the Routine-class check on bpmux
///   content because `session::classify` returns Routine for
///   ANY bpmux-sender message, which would hide tool output.
pub(super) fn should_hide_by_class(m: &session::Message, owner_name: &str) -> bool {
    if m.message_type == session::MessageType::Nudge {
        return true;
    }
    let sender = m.from.split(':').next().unwrap_or(&m.from);
    if sender == "bpmux" {
        return m.text.starts_with('\u{1F4DD}');
    }
    matches!(
        session::classify(m, owner_name),
        session::MessageClass::Routine
    )
}

pub(super) fn render_chat(frame: &mut Frame, app: &mut App, area: Rect) {
    // Channel-view filter (existing behavior) + class filter (#161).
    // Imperative loop so we can both collect visible messages AND count
    // the hidden ones for the status bar's "N hidden" indicator.
    let mut chat_messages: Vec<&session::Message> = Vec::new();
    let mut hidden_class_count: usize = 0;
    for m in &app.messages {
        // Non-chat messages: system/activity, governed by show_system_messages
        if !m.is_chat() {
            if app.show_system_messages {
                chat_messages.push(m);
            }
            continue;
        }
        // Channel filter — skip messages that aren't in the current view.
        let in_channel = match &app.current_view {
            None => true,
            Some(view) if view == "broadcast" => {
                app::channel_name(m, &app.peer_name) == "broadcast"
            }
            Some(view) => message_matches_channel(m, view, &app.peer_name),
        };
        if !in_channel {
            continue;
        }
        // #161 / #186 class filter. See `should_hide_by_class` below
        // for the full rule set. Extracted into a pure function so
        // the AC can be unit-tested against each rule branch
        // without needing to drive the full render loop.
        if !app.show_all_classes && should_hide_by_class(m, &app.peer_name) {
            hidden_class_count += 1;
            continue;
        }
        // #181 focus-mode display filter. Orthogonal to the classifier
        // filter above — both can be active. The pure function
        // `in_focus` handles the override rules (own messages, system
        // messages, addressee=my_role bypass).
        if !super::commands_focus::in_focus(m, app.focused_roles.as_deref(), &app.peer_name) {
            continue;
        }
        chat_messages.push(m);
    }
    app.hidden_class_count = hidden_class_count;
    let chat_inner_width = area.width.saturating_sub(2) as usize;
    let chat_height = area.height.saturating_sub(2) as usize;

    let visible_window = chat_height * 3;
    let total_msgs = chat_messages.len();
    let scroll_back = app.scroll_offset;
    let needed = visible_window + scroll_back;
    let start_idx = total_msgs.saturating_sub(needed);

    let mut truncation_line_indices: Vec<(usize, String)> = Vec::new();
    let mut messages: Vec<Line> = Vec::new();

    for (i, msg) in chat_messages.iter().enumerate().skip(start_idx) {
        let short_name = msg.from.split(':').next().unwrap_or(&msg.from);
        let is_human = msg.from.ends_with(":human");
        let name_color = sender_color(short_name);
        let ch_name = app::channel_name(msg, &app.peer_name);
        let is_broadcast = ch_name == "broadcast";
        let channel_tag = format!("[{ch_name}] ");

        let mut prefix = String::new();
        if app.show_timestamps {
            let time = msg.ts.format("%H:%M:%S").to_string();
            prefix.push_str(&format!("[{time}] "));
        }
        prefix.push_str(&channel_tag);
        prefix.push_str(&format!("{short_name}: "));

        let full_len = prefix.len() + msg.text.len();
        let wrapped_lines = if chat_inner_width > 0 {
            full_len.max(1).div_ceil(chat_inner_width)
        } else {
            1
        };

        let ts_style = Style::default().fg(DRACULA_COMMENT);
        let tag_color = channel_tag_color(&channel_tag, is_broadcast);
        let tag_style = Style::default().fg(tag_color);
        let msg_key = msg.ts.to_rfc3339();

        let name_style = if is_human {
            Style::default().fg(name_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(name_color)
        };
        let indent = Span::raw(" ".repeat(prefix.len()));
        let mut prefix_spans = Vec::new();
        if app.show_timestamps {
            let time = msg.ts.format("%H:%M:%S").to_string();
            prefix_spans.push(Span::styled(format!("[{time}] "), ts_style));
        }
        prefix_spans.push(Span::styled(channel_tag.clone(), tag_style));
        if msg.message_type.is_insight() {
            prefix_spans.push(Span::styled(
                format!("{short_name} ★ "),
                Style::default().fg(DRACULA_COMMENT),
            ));
        } else {
            prefix_spans.push(Span::styled(format!("{short_name}: "), name_style));
        }

        let is_long = wrapped_lines > TRUNCATE_AFTER_LINES;
        let is_expanded = app.expanded_messages.contains(&msg_key);
        let is_truncated = is_long && !is_expanded;

        let md_lines = if is_truncated {
            let remaining_lines = wrapped_lines.saturating_sub(7);
            let mut ls = render_md_message(
                &msg.text,
                prefix_spans,
                indent.clone(),
                Some(7),
                chat_inner_width,
            );
            // #183: `[+N lines]` gets the clickable-accent treatment
            // (cyan + underline) so the weak bracket-affordance becomes
            // an obvious call-to-action. The truncation_indicators
            // registry (populated below) already tracks which row each
            // indicator renders on, so the click handler continues to
            // work unchanged.
            ls.push(Line::from(Span::styled(
                format!("  [+{remaining_lines} lines]"),
                Style::default()
                    .fg(DRACULA_CYAN)
                    .add_modifier(Modifier::UNDERLINED),
            )));
            ls
        } else {
            render_md_message(
                &msg.text,
                prefix_spans,
                indent.clone(),
                None,
                chat_inner_width,
            )
        };

        for line in
            md_lines.iter().take(
                md_lines
                    .len()
                    .saturating_sub(if is_truncated { 1 } else { 0 }),
            )
        {
            messages.extend(wrap_line_with_indent(
                line.clone(),
                &indent,
                chat_inner_width,
            ));
        }

        if is_truncated && let Some(indicator_line) = md_lines.last() {
            truncation_line_indices.push((messages.len(), msg_key.clone()));
            messages.push(indicator_line.clone());
        } else if is_long && is_expanded {
            truncation_line_indices.push((messages.len(), msg_key));
            messages.push(Line::from(Span::styled(
                "  [-collapse]",
                Style::default().fg(DRACULA_COMMENT),
            )));
        }

        let next_is_different = chat_messages
            .get(i + 1)
            .is_some_and(|next| next.from != msg.from);

        if next_is_different {
            messages.push(Line::default());
        }
    }

    let total_lines = messages.len();
    // #183: prefix the title with a `▸`/`▾` toggle glyph so the
    // click-to-expand/collapse affordance is visible. `▸` when the
    // project view is collapsed (click to expand), `▾` when open
    // (click to collapse). Glyph vocabulary shared with the icons
    // module for consistency across surfaces.
    let toggle_glyph = if app.show_project_view {
        super::icons::COLLAPSE
    } else {
        super::icons::EXPAND
    };
    let title_body = app
        .project_state
        .as_ref()
        .map_or(" bpmux ".to_string(), |s| s.title_summary());
    // `title_body` already has leading/trailing spaces; splice the
    // glyph in after the leading space so rendering stays clean.
    let title = if let Some(rest) = title_body.strip_prefix(' ') {
        format!(" {toggle_glyph} {rest}")
    } else {
        format!("{toggle_glyph} {title_body}")
    };
    let chat_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(DRACULA_CYAN)
                .add_modifier(Modifier::UNDERLINED),
        )
        .border_style(Style::default().fg(DRACULA_SELECTION))
        .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));

    // #180 (tui-scrollview): render the block first to get the inner
    // area, then render message content into a ScrollView. The
    // ScrollViewState manages viewport position as an absolute row
    // offset — no compensation math needed when new messages arrive
    // or when filtered messages land.
    let inner = chat_block.inner(area);
    frame.render_widget(chat_block, area);

    #[allow(clippy::cast_possible_truncation)]
    let content_height = total_lines.max(1) as u16;
    let mut scroll_view =
        tui_scrollview::ScrollView::new(ratatui::layout::Size::new(inner.width, content_height));
    scroll_view.render_widget(
        Paragraph::new(messages).style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
        Rect::new(0, 0, inner.width, content_height),
    );

    // If at bottom, ensure the state tracks the latest content
    // height so scroll_to_bottom in receive_message stays accurate.
    if app.chat_at_bottom {
        app.chat_scroll_state.scroll_to_bottom();
    }

    frame.render_stateful_widget(scroll_view, inner, &mut app.chat_scroll_state);

    // Update max_scroll for legacy status_bar display.
    let max_scroll = total_lines.saturating_sub(chat_height);
    app.max_scroll = max_scroll;

    // Truncation indicator tracking — compute screen positions from
    // the ScrollViewState offset rather than the old scroll variable.
    let scroll_y = app.chat_scroll_state.offset().y as usize;
    let inner_y = inner.y;
    app.truncation_indicators.clear();
    for (line_idx, key) in &truncation_line_indices {
        if *line_idx >= scroll_y && *line_idx < scroll_y + chat_height {
            #[allow(clippy::cast_possible_truncation)]
            let screen_y = inner_y + (*line_idx - scroll_y) as u16;
            app.truncation_indicators.push((screen_y, key.clone()));
        }
    }
}

pub(super) fn render_unread_indicator(frame: &mut Frame, app: &App, area: Rect) {
    if !app.chat_at_bottom && app.unread_count > 0 {
        let unread_text = format!(
            "\u{2193} {} new message{}",
            app.unread_count,
            if app.unread_count == 1 { "" } else { "s" }
        );
        let unread_line = Line::from(Span::styled(
            unread_text,
            Style::default()
                .fg(DRACULA_PURPLE)
                .add_modifier(Modifier::BOLD),
        ));
        let unread_area = Rect::new(
            area.x + 1,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(2),
            1,
        );
        frame.render_widget(Paragraph::new(unread_line), unread_area);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for `should_hide_by_class` — #161 / #186 class
    //! filter predicate. Pure function tests named for what the
    //! function returns, per the test-name discipline.

    use super::*;
    use crate::session::{Message, MessageType};
    use chrono::Utc;

    fn msg(from: &str, text: &str, ty: MessageType) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: ty,
            to: None,
            addressee: None,
            explicit_privacy: false,
            multi_depth: None,
        }
    }

    #[test]
    fn should_hide_returns_true_for_nudge_type_regardless_of_sender_or_text() {
        // #186 load-bearing case: the handoff liveness warning that
        // escaped the pre-#186 filter because it didn't start with
        // 📝. Under the new type-based filter it's hidden by type.
        let handoff_warning = msg(
            "bpmux",
            "dev hasn't acknowledged the handoff from pm. Waiting for response.",
            MessageType::Nudge,
        );
        assert!(should_hide_by_class(&handoff_warning, "ryan"));

        // Nudge type wins over content — even a message with arbitrary
        // text is hidden if it's typed as Nudge.
        let arbitrary = msg("bpmux", "anything at all", MessageType::Nudge);
        assert!(should_hide_by_class(&arbitrary, "ryan"));
    }

    #[test]
    fn should_hide_returns_true_for_legacy_pencil_prefixed_bpmux_chat() {
        // Backward compat: historical sessions logged nudges as
        // Chat with the 📝 prefix. The filter still catches them.
        let legacy_nudge = msg(
            "bpmux",
            "\u{1F4DD} 2 candidate moments from recent chat",
            MessageType::Chat,
        );
        assert!(should_hide_by_class(&legacy_nudge, "ryan"));
    }

    #[test]
    fn should_hide_returns_false_for_bpmux_tool_output_without_pencil_prefix() {
        // AC #6: the filter must NOT hide tool output. `/board`,
        // `/roadmap`, `/who`, `/debug`, and `/focus` all go through
        // `App::system_message` which creates bpmux-sender chat
        // messages. Hiding these would break the user-facing tool
        // response loop.
        let board_output = msg("bpmux", "Roadmap: ...", MessageType::Chat);
        assert!(!should_hide_by_class(&board_output, "ryan"));

        let who_output = msg("bpmux", "Participants:\n  ryan (you)", MessageType::Chat);
        assert!(!should_hide_by_class(&who_output, "ryan"));

        let focus_output = msg("bpmux", "focus: off (all roles visible)", MessageType::Chat);
        assert!(!should_hide_by_class(&focus_output, "ryan"));
    }

    #[test]
    fn should_hide_returns_false_for_non_bpmux_pencil_prefixed_message() {
        // AC #6 prevent-overcorrection case: a message from another
        // role that happens to start with 📝 must NOT be hidden —
        // only sender=bpmux gets the prefix-based fallback. A dev
        // broadcast about a checklist shouldn't accidentally trip
        // the nudge filter.
        let dev_message = msg(
            "dev:claude-code",
            "\u{1F4DD} TODO list for #186",
            MessageType::Chat,
        );
        assert!(!should_hide_by_class(&dev_message, "ryan"));
    }

    #[test]
    fn should_hide_returns_false_for_regular_peer_chat_message() {
        // Normal peer chat from pm/designer/dev is not filtered.
        let pm = msg(
            "pm:claude-code",
            "what does the queue look like?",
            MessageType::Chat,
        );
        assert!(!should_hide_by_class(&pm, "ryan"));
    }
}
