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
        // #161 class filter: when `show_all_classes` is off, hide
        // (a) classifier auto-nudges (bpmux-sent messages whose text
        //     starts with the 📝 marker introduced by #163) and
        // (b) non-bpmux messages that the classifier returns as Routine.
        //
        // Important: bpmux sends MORE than just auto-nudges — tool
        // output (/board, /roadmap, /who, /debug), system notices,
        // error messages, peer join/leave announcements. The user
        // wants to see all of those. Only the 📝-prefixed auto-nudges
        // should be suppressed. And because `session::classify` itself
        // returns Routine for ANY bpmux-sender message, we must NOT
        // run the Routine-class check on bpmux content — otherwise we
        // hide all bpmux output, not just the nudges.
        //
        // Hidden messages stay in `app.messages` and become visible
        // when the user toggles via `/all`. Classifier cost per render
        // is microseconds via the thread-local Aho-Corasick cache.
        if !app.show_all_classes {
            let sender = m.from.split(':').next().unwrap_or(&m.from);
            if sender == "bpmux" {
                if m.text.starts_with('\u{1F4DD}') {
                    hidden_class_count += 1;
                    continue;
                }
                // Non-nudge bpmux output (tool results, system notices)
                // always renders — don't run the Routine-class check
                // because classify always returns Routine for bpmux.
            } else if matches!(
                session::classify(m, &app.peer_name),
                session::MessageClass::Routine
            ) {
                hidden_class_count += 1;
                continue;
            }
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
    let chat = Paragraph::new(messages).block(chat_block);
    let max_scroll = total_lines.saturating_sub(chat_height);
    app.max_scroll = max_scroll;
    #[allow(clippy::cast_possible_truncation)]
    let scroll = max_scroll
        .saturating_sub(app.scroll_offset)
        .min(u16::MAX as usize) as u16;

    let inner_y = area.y + 1;
    let scroll_usize = scroll as usize;
    app.truncation_indicators.clear();
    for (line_idx, key) in &truncation_line_indices {
        if *line_idx >= scroll_usize && *line_idx < scroll_usize + chat_height {
            #[allow(clippy::cast_possible_truncation)]
            let screen_y = inner_y + (*line_idx - scroll_usize) as u16;
            app.truncation_indicators.push((screen_y, key.clone()));
        }
    }

    let chat = chat.scroll((scroll, 0));
    frame.render_widget(chat, area);
}

pub(super) fn render_unread_indicator(frame: &mut Frame, app: &App, area: Rect) {
    if app.scroll_offset > 0 && app.unread_count > 0 {
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
