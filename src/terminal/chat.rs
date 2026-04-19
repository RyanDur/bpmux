//! Chat area rendering — message filtering, truncation, and display.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::session::{self, message::DeepDetail};
use crate::terminal::app::{self, App, TRUNCATE_AFTER_LINES};
use crate::terminal::app_state::DepthLevel;
use crate::terminal::markdown::{render_md_message, wrap_line_with_indent};
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
    // Rebuild filtered index only when dirty (message arrival,
    // filter change), not every frame.
    app.rebuild_filtered_if_dirty();
    let chat_messages: Vec<&session::Message> = app
        .msg
        .filtered_indices
        .iter()
        .filter_map(|&idx| app.msg.messages.get(idx))
        .collect();
    app.display.hidden_class_count = app.msg.cached_hidden_count;
    let chat_inner_width = area.width.saturating_sub(2) as usize;
    let chat_height = area.height.saturating_sub(2) as usize;

    let visible_window = chat_height * 3;
    let total_msgs = chat_messages.len();
    let scroll_back = app.scroll.offset;
    let needed = visible_window + scroll_back;
    let start_idx = total_msgs.saturating_sub(needed);

    let mut truncation_line_indices: Vec<(usize, String)> = Vec::new();
    let mut depth_line_indices: Vec<(usize, String)> = Vec::new();
    let mut messages: Vec<Line> = Vec::new();
    // Short role name for multi-depth role-lookup (#285). `identity.name`
    // may include a `:agent` suffix (e.g. `dev:claude-code`); strip so
    // the role key matches `MultiDepthPayload.role_default` entries.
    let viewer_role: String = app
        .identity
        .name
        .split(':')
        .next()
        .unwrap_or(&app.identity.name)
        .to_string();

    for (i, msg) in chat_messages.iter().enumerate().skip(start_idx) {
        let short_name = msg.from.split(':').next().unwrap_or(&msg.from);
        let is_human = msg.from.ends_with(":human");
        let name_color = sender_color(short_name);
        let ch_name = app::channel_name(msg, &app.identity.name);
        let is_broadcast = ch_name == "broadcast";
        let channel_tag = format!("[{ch_name}] ");

        let mut prefix = String::new();
        if app.display.show_timestamps {
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
        if app.display.show_timestamps {
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

        // Multi-depth branch (#285): when a message carries a
        // MultiDepthPayload, render the depth-appropriate variant
        // instead of msg.text (AC: msg.text is suppressed when
        // multi_depth is present, prevent double-render). A cyan-
        // underlined cycle affordance at the end is registered in
        // `depth_line_indices` so the click handler can cycle depth.
        if let Some(payload) = msg.multi_depth.as_ref() {
            let current_depth = app
                .msg
                .expanded_messages
                .get(&msg_key)
                .copied()
                .unwrap_or_else(|| DepthLevel::initial_for_role(&viewer_role));
            let content: String = match current_depth {
                DepthLevel::Summary => payload.summary.clone(),
                DepthLevel::RoleDefault => payload
                    .role_default
                    .get(&viewer_role)
                    .cloned()
                    .unwrap_or_else(|| payload.summary.clone()),
                DepthLevel::Deep => match &payload.deep {
                    DeepDetail::Preloaded { text } => text.clone(),
                    // OnDemand fetch is not wired in v1 (#285 scope
                    // per 2026-04-17 shape converge — deferred to a
                    // follow-up). Render a non-clickable placeholder
                    // so the affordance is visible without invoking
                    // a fetch mechanism that doesn't exist yet.
                    DeepDetail::OnDemand {
                        command,
                        description,
                    } => format!("[deep detail: {description} — run `{command}`]"),
                },
            };
            // Build owned Lines directly (bypassing render_md_message's
            // 'a-bound-to-text lifetime so we can store local `content`
            // in the render output). Multi-depth content is already the
            // final text — no markdown re-parse needed. The first line
            // carries the prefix spans; subsequent lines are indented.
            let content_lines: Vec<String> = content
                .lines()
                .map(std::string::ToString::to_string)
                .collect();
            if let Some((first, rest)) = content_lines.split_first() {
                let mut first_spans = prefix_spans;
                first_spans.push(Span::raw(first.clone()));
                messages.extend(wrap_line_with_indent(
                    Line::from(first_spans),
                    &indent,
                    chat_inner_width,
                ));
                for line_text in rest {
                    messages.extend(wrap_line_with_indent(
                        Line::from(vec![indent.clone(), Span::raw(line_text.clone())]),
                        &indent,
                        chat_inner_width,
                    ));
                }
            } else {
                // Empty content (summary/role_default empty) — still
                // emit the prefix so the message row renders.
                messages.extend(wrap_line_with_indent(
                    Line::from(prefix_spans),
                    &indent,
                    chat_inner_width,
                ));
            }
            // Cycle affordance label — at Summary/RoleDefault a click
            // advances deeper; at Deep a click wraps back to Summary.
            // Dim-prefix + cyan-underlined pattern consistent with
            // #110's `[+N lines]` so users recognize click-to-toggle.
            let affordance_label = match current_depth {
                DepthLevel::Summary | DepthLevel::RoleDefault => "[↓ detail]",
                DepthLevel::Deep => "[↻ summary]",
            };
            depth_line_indices.push((messages.len(), msg_key.clone()));
            messages.push(Line::from(Span::styled(
                format!("  {affordance_label}"),
                Style::default()
                    .fg(DRACULA_CYAN)
                    .add_modifier(Modifier::UNDERLINED),
            )));
            let next_is_different = chat_messages
                .get(i + 1)
                .is_some_and(|next| next.from != msg.from);
            if next_is_different {
                messages.push(Line::default());
            }
            continue;
        }

        let is_long = wrapped_lines > TRUNCATE_AFTER_LINES;
        let is_expanded = app.msg.expanded_messages.contains_key(&msg_key);
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
            // `[+N lines]` gets the clickable-accent treatment
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
    // Prefix the title with a `▸`/`▾` toggle glyph so the
    // click-to-expand/collapse affordance is visible. `▸` when the
    // project view is collapsed (click to expand), `▾` when open
    // (click to collapse). Glyph vocabulary shared with the icons
    // module for consistency across surfaces.
    let toggle_glyph = if matches!(app.input_mode, crate::terminal::app::InputMode::Project) {
        super::icons::COLLAPSE
    } else {
        super::icons::EXPAND
    };
    let title_body = app
        .project
        .state
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
    let chat = Paragraph::new(messages).block(chat_block);
    let max_scroll = total_lines.saturating_sub(chat_height);
    app.scroll.max = max_scroll;
    #[allow(clippy::cast_possible_truncation)]
    let scroll = max_scroll
        .saturating_sub(app.scroll.offset)
        .min(u16::MAX as usize) as u16;

    let inner_y = area.y + 1;
    let scroll_usize = scroll as usize;
    app.msg.truncation_indicators.clear();
    for (line_idx, key) in &truncation_line_indices {
        if *line_idx >= scroll_usize && *line_idx < scroll_usize + chat_height {
            #[allow(clippy::cast_possible_truncation)]
            let screen_y = inner_y + (*line_idx - scroll_usize) as u16;
            app.msg.truncation_indicators.push((screen_y, key.clone()));
        }
    }
    app.msg.depth_indicators.clear();
    for (line_idx, key) in &depth_line_indices {
        if *line_idx >= scroll_usize && *line_idx < scroll_usize + chat_height {
            #[allow(clippy::cast_possible_truncation)]
            let screen_y = inner_y + (*line_idx - scroll_usize) as u16;
            app.msg.depth_indicators.push((screen_y, key.clone()));
        }
    }

    let chat = chat.scroll((scroll, 0));
    frame.render_widget(chat, area);
}

pub(super) fn render_unread_indicator(frame: &mut Frame, app: &App, area: Rect) {
    if app.scroll.offset > 0 && app.scroll.unread_count > 0 {
        let unread_text = format!(
            "{} {} new message{}",
            super::icons::DOWN,
            app.scroll.unread_count,
            if app.scroll.unread_count == 1 {
                ""
            } else {
                "s"
            }
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
            ..Message::default()
        }
    }

    #[test]
    fn should_hide_returns_true_for_nudge_type_regardless_of_sender_or_text() {
        // Regression guard: the handoff liveness warning used to
        // escape the text-prefix filter because it didn't start with
        // 📝. Under the type-based filter it's hidden by type.
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
