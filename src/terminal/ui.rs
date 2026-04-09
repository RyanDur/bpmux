use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::session;
use crate::terminal::app::{self, App};

const TRUNCATE_AFTER_LINES: usize = 10;
const SIDEBAR_NAME_WIDTH: usize = 16;
const MIN_WIDTH_FOR_SIDEBAR: u16 = 60;
use crate::terminal::markdown::{
    message_matches_channel, render_md_message, wrap_line_with_indent,
};
use crate::terminal::theme::{
    DRACULA_BG, DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_GREEN, DRACULA_ORANGE,
    DRACULA_PINK, DRACULA_PURPLE, DRACULA_RED, DRACULA_SELECTION, DRACULA_YELLOW,
    channel_tag_color, sender_color,
};

fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let channels = app.sidebar_channels();
    let items: Vec<Line> = channels
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let selected = i == app.sidebar_selected && app.sidebar_focused;
            let bg = if selected {
                DRACULA_SELECTION // Dracula selection
            } else {
                DRACULA_BG // Dracula bg
            };

            // Index 0 = project root node
            if i == 0 {
                let arrow = if app.project_expanded { "▾ " } else { "▸ " };
                let is_all = app.current_view.is_none();
                let name_style = if is_all {
                    Style::default()
                        .fg(DRACULA_FG)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DRACULA_FG).bg(bg)
                };
                let select_marker = if selected { "▸ " } else { "  " };
                return Line::from(vec![
                    Span::styled(select_marker, Style::default().bg(bg).fg(DRACULA_PURPLE)),
                    Span::styled(arrow, Style::default().bg(bg).fg(DRACULA_COMMENT)),
                    Span::styled(
                        ch.chars().take(SIDEBAR_NAME_WIDTH).collect::<String>(),
                        name_style,
                    ),
                ]);
            }

            // Channel entries — indented under project
            let is_current = app.current_view.as_deref() == Some(ch.as_str());
            // Suppress unread count on broadcast — it's ambient, visible from most views
            let unread = if ch == "broadcast" {
                0
            } else {
                app.channel_unread.get(ch).copied().unwrap_or(0)
            };

            let select_marker = if selected { "▸ " } else { "  " };
            // Prefix: select(2) + indent(2) = 4 chars; border uses 2
            let inner_width = area.width.saturating_sub(2) as usize;
            let prefix_len = 4;
            let available = inner_width.saturating_sub(prefix_len);

            // Unread count badge takes priority over channel name length
            let badge = if unread > 0 {
                format!(" {unread}")
            } else {
                String::new()
            };
            let name_budget = available.saturating_sub(badge.len());
            let display_name: String = if ch.len() > name_budget {
                format!(
                    "{}…",
                    ch.chars()
                        .take(name_budget.saturating_sub(1))
                        .collect::<String>()
                )
            } else {
                ch.clone()
            };

            let name_style = if is_current || unread > 0 {
                Style::default()
                    .fg(DRACULA_FG)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DRACULA_COMMENT).bg(bg)
            };

            let mut spans = vec![
                Span::styled(select_marker, Style::default().bg(bg).fg(DRACULA_PURPLE)),
                Span::styled("  ", Style::default().bg(bg)), // indent under project
                Span::styled(display_name, name_style),
            ];
            if unread > 0 {
                spans.push(Span::styled(
                    badge,
                    Style::default()
                        .bg(bg)
                        .fg(DRACULA_CYAN)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Line::from(spans)
        })
        .collect();

    let border_color = if app.sidebar_focused {
        DRACULA_PURPLE // Dracula purple — focused
    } else {
        DRACULA_SELECTION // Dracula selection — unfocused
    };

    let sidebar = Paragraph::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(if app.sidebar_filter.is_empty() {
                " channels ".to_string()
            } else {
                format!(" filter: {} ", app.sidebar_filter)
            })
            .title_style(
                Style::default()
                    .fg(DRACULA_PURPLE)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
    );
    frame.render_widget(sidebar, area);
}

const MAX_BANNER_LINES: u16 = 4;

fn latest_broadcast(app: &App) -> Option<&session::Message> {
    app.messages
        .iter()
        .rev()
        .find(|m| m.is_chat() && m.to.is_none())
}

fn broadcast_banner_height(app: &App, width: u16) -> u16 {
    if app.current_view.as_ref().is_none_or(|v| v == "broadcast") {
        return 0;
    }
    let Some(msg) = latest_broadcast(app) else {
        return 0;
    };
    let sender = msg.from.split(':').next().unwrap_or(&msg.from);
    let prefix_len = format!("[broadcast] {sender}: ").chars().count();
    let content_len = prefix_len + msg.text.chars().count();
    let w = width as usize;
    if w == 0 {
        return 1;
    }
    (content_len.max(1).div_ceil(w) as u16).min(MAX_BANNER_LINES)
}

fn render_broadcast_banner(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    let Some(msg) = latest_broadcast(app) else {
        return;
    };
    let sender = msg.from.split(':').next().unwrap_or(&msg.from);
    let prefix = format!("[broadcast] {sender}: ");
    let text = Line::from(vec![
        Span::styled(&prefix, Style::default().fg(DRACULA_COMMENT)),
        Span::styled(msg.text.as_str(), Style::default().fg(DRACULA_FG)),
    ]);
    let banner = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(DRACULA_SELECTION).fg(DRACULA_FG));
    frame.render_widget(banner, area);
}

fn render_chat(frame: &mut Frame, app: &mut App, area: Rect) {
    let chat_messages: Vec<&session::Message> = app
        .messages
        .iter()
        .filter(|m| {
            if !m.is_chat() {
                return app.show_system_messages;
            }
            match &app.current_view {
                None => true,
                Some(view) if view == "broadcast" => {
                    app::channel_name(m, &app.peer_name) == "broadcast"
                }
                Some(view) => message_matches_channel(m, view, &app.peer_name),
            }
        })
        .collect();
    let chat_inner_width = area.width.saturating_sub(2) as usize;
    let chat_height = area.height.saturating_sub(2) as usize;

    // Virtualization: only render messages near the visible window
    let visible_window = chat_height * 3; // generous buffer for scroll
    let total_msgs = chat_messages.len();
    let scroll_back = app.scroll_offset;
    let needed = visible_window + scroll_back;
    let start_idx = total_msgs.saturating_sub(needed);

    // Track (line_index, message_key) for each "[+N lines]" indicator
    let mut truncation_line_indices: Vec<(usize, String)> = Vec::new();

    let mut messages: Vec<Line> = Vec::new();
    for (i, msg) in chat_messages.iter().enumerate().skip(start_idx) {
        // Short name: "ryan:human" -> "ryan", "claude-code:ws" -> "claude-code"
        let short_name = msg.from.split(':').next().unwrap_or(&msg.from);

        // Deterministic color from sender name — same name always gets same color
        let is_human = msg.from.ends_with(":human");
        let name_color = sender_color(short_name);

        // Channel tag: [broadcast], [dev], [dev+pm] — owner excluded
        let ch_name = app::channel_name(msg, &app.peer_name);
        let is_broadcast = ch_name == "broadcast";
        let channel_tag = format!("[{ch_name}] ");

        // Build the prefix (timestamp + channel tag + name)
        let mut prefix = String::new();
        if app.show_timestamps {
            let time = msg.ts.format("%H:%M:%S").to_string();
            prefix.push_str(&format!("[{time}] "));
        }
        prefix.push_str(&channel_tag);
        prefix.push_str(&format!("{short_name}: "));

        // Estimate wrapped line count for truncation decision
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
            ls.push(Line::from(Span::styled(
                format!("  [+{remaining_lines} lines]"),
                Style::default().fg(DRACULA_COMMENT),
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

        // Add blank line separator only when the next message is from a different sender
        let next_is_different = chat_messages
            .get(i + 1)
            .is_some_and(|next| next.from != msg.from);

        if next_is_different {
            messages.push(Line::default());
        }
    }

    // Lines are pre-wrapped by wrap_line_with_indent — no ratatui Wrap needed
    let total_lines = messages.len();
    let title = app
        .project_state
        .as_ref()
        .map_or(" bpmux ".to_string(), |s| s.title_summary());
    let chat_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(DRACULA_PURPLE))
        .border_style(Style::default().fg(DRACULA_SELECTION))
        .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));
    let chat = Paragraph::new(messages).block(chat_block);
    let max_scroll = total_lines.saturating_sub(chat_height);
    app.max_scroll = max_scroll;
    #[allow(clippy::cast_possible_truncation)]
    let scroll = max_scroll
        .saturating_sub(app.scroll_offset)
        .min(u16::MAX as usize) as u16;

    // Map truncation indicator line indices to screen Y positions
    let inner_y = area.y + 1; // +1 for top border
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

fn render_unread_indicator(frame: &mut Frame, app: &App, area: Rect) {
    if app.scroll_offset > 0 && app.unread_count > 0 {
        let unread_text = format!(
            "\u{2193} {} new message{}",
            app.unread_count,
            if app.unread_count == 1 { "" } else { "s" }
        );
        let unread_width = unread_text.len() as u16 + 2; // +2 for padding
        let unread_x = area.x + (area.width.saturating_sub(unread_width)) / 2;
        let unread_y = area.y + area.height.saturating_sub(2);
        let unread_area = Rect::new(unread_x, unread_y, unread_width, 1);
        let unread = Paragraph::new(Span::styled(
            format!(" {unread_text} "),
            Style::default().fg(DRACULA_BG).bg(DRACULA_CYAN),
        ));
        frame.render_widget(Clear, unread_area);
        frame.render_widget(unread, unread_area);
    }
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let input_line = if app.input.starts_with('/') {
        // Color the command keyword differently from the arguments
        let cmd_end = app.input.find(' ').unwrap_or(app.input.len());
        let cmd = &app.input[..cmd_end];
        let rest = &app.input[cmd_end..];
        Line::from(vec![
            Span::styled(
                cmd.to_string(),
                Style::default().fg(DRACULA_PURPLE), // Dracula purple
            ),
            Span::styled(
                rest.to_string(),
                Style::default().fg(DRACULA_FG), // Default fg
            ),
        ])
    } else {
        Line::from(app.input.as_str())
    };
    let input = Paragraph::new(input_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(match &app.send_target {
                    Some(ch) => format!(" {} [{ch}] > ", app.peer_name),
                    None => format!(" {} > ", app.peer_name),
                })
                .title_style(Style::default().fg(DRACULA_PURPLE))
                .border_style(Style::default().fg(if app.sidebar_focused {
                    DRACULA_SELECTION
                } else {
                    DRACULA_PURPLE
                }))
                .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(input, area);

    // Show cursor at end of input text, accounting for wrapping
    #[allow(clippy::cast_possible_truncation)]
    let input_inner_width = area.width.saturating_sub(2) as usize;
    let display_pos = app.input[..app.cursor_pos].chars().count();
    let (cursor_x, cursor_y) = if input_inner_width > 0 {
        let x = (display_pos % input_inner_width) as u16;
        let y = (display_pos / input_inner_width) as u16;
        (area.x + 1 + x, area.y + 1 + y)
    } else {
        (area.x + 1, area.y + 1)
    };
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_popup(frame: &mut Frame, app: &App, input_area: Rect) {
    let matches = app.popup_matches();
    if !matches.is_empty() {
        let popup_height = matches.len() as u16 + 2; // +2 for borders
        let popup_width = 40u16.min(input_area.width);
        // Position popup above the input pane
        let popup_y = input_area.y.saturating_sub(popup_height);
        let popup_area = Rect::new(input_area.x, popup_y, popup_width, popup_height);

        let items: Vec<Line> = matches
            .iter()
            .enumerate()
            .map(|(i, (cmd, desc))| {
                let selected = app.popup_selected == Some(i);
                let (cmd_style, desc_style) = if selected {
                    (
                        Style::default().fg(DRACULA_FG).bg(DRACULA_SELECTION),
                        Style::default().fg(DRACULA_PURPLE).bg(DRACULA_SELECTION),
                    )
                } else {
                    (
                        Style::default().fg(DRACULA_PURPLE),
                        Style::default().fg(DRACULA_COMMENT),
                    )
                };
                Line::from(vec![
                    Span::styled(format!(" {cmd:<10}"), cmd_style),
                    Span::styled((*desc).to_string(), desc_style),
                ])
            })
            .collect();
        let popup = Paragraph::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DRACULA_COMMENT))
                .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
        );
        frame.render_widget(Clear, popup_area);
        frame.render_widget(popup, popup_area);
    }
}

fn render_status_bar(frame: &mut Frame, app: &mut App, area: Rect) {
    let connection_status = if app.peer_activity.is_empty() {
        Span::styled(" solo", Style::default().fg(DRACULA_ORANGE))
    } else {
        Span::raw("")
    };
    let view_label = match &app.current_view {
        None => " [all]".to_string(),
        Some(v) if v == "broadcast" => " [broadcast]".to_string(),
        Some(v) => format!(" [{v}]"),
    };
    let mut status_spans = vec![
        Span::styled(view_label, Style::default().fg(DRACULA_CYAN)),
        Span::raw(" "),
        connection_status,
    ];
    if app.scroll_offset > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("scrolled up {}", app.scroll_offset),
            Style::default().fg(DRACULA_COMMENT),
        ));
    }
    if app.unread_count > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("{} new", app.unread_count),
            Style::default().fg(DRACULA_PINK),
        ));
    }
    let mut peers: Vec<_> = app.peer_activity.iter().collect();
    peers.sort_by_key(|(name, _)| (*name).clone());
    for (peer, (status, _)) in &peers {
        let dot_color = match status.as_str() {
            "busy" => DRACULA_YELLOW,
            "help" => DRACULA_RED,
            _ => DRACULA_GREEN,
        };
        status_spans.push(Span::raw(" "));
        status_spans.push(Span::styled("\u{25CF}", Style::default().fg(dot_color)));
        status_spans.push(Span::styled(
            format!(" {peer}"),
            Style::default().fg(DRACULA_FG),
        ));
    }
    // DM unread badge with sender names
    if !app.dm_unread_from.is_empty() {
        let names = app.dm_unread_from.join(", ");
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("\u{2709} DM from {names}"),
            Style::default().fg(DRACULA_YELLOW), // Dracula yellow
        ));
    }
    // Right-aligned "? help" hint
    let used: usize = status_spans.iter().map(|s| s.content.chars().count()).sum();
    let bar_width = area.width as usize;
    let hint = if app.sidebar_focused {
        "Enter=switch \u{2191}\u{2193}=nav Esc=close"
    } else {
        "?=help \u{2325}1=sidebar Enter=send"
    };
    let right_margin = 1;
    if used + hint.chars().count() + right_margin + 1 < bar_width {
        let pad = bar_width.saturating_sub(used + hint.chars().count() + right_margin);
        app.help_hint_col = Some(area.x + (used + pad) as u16);
        status_spans.push(Span::raw(" ".repeat(pad)));
        status_spans.push(Span::styled(hint, Style::default().fg(DRACULA_CYAN)));
    } else {
        app.help_hint_col = None;
    }
    let status = Paragraph::new(Line::from(status_spans))
        .style(Style::default().bg(DRACULA_SELECTION).fg(DRACULA_FG));
    frame.render_widget(status, area);
}

fn render_help_overlay(frame: &mut Frame, app: &App, area: Rect) {
    if app.show_help {
        let help_text = "\
Commands:
  /to <who> <msg>   send to recipients
  /view [channel]   filter by channel
  /board   project board   /who    participants
  /session create P2P      /join   join remote
  /accept  complete P2P    /status set status
  /debug   system msgs     /quit   exit

Keys:
  Enter    send / expand    Esc     clear input
  Ctrl+U   scroll up        Ctrl+D  scroll down
  \u{2325}1       channel sidebar   Ctrl+T  timestamps
  Ctrl+C   quit (empty)     ?  this overlay

Press any key to close";

        let help_lines: Vec<Line> = help_text
            .lines()
            .map(|l| Line::from(Span::raw(l.to_string())))
            .collect();
        let help_height = (help_lines.len() as u16 + 2).min(area.height); // +2 for borders
        let help_width = 52u16.min(area.width);
        let help_x = area.x + (area.width.saturating_sub(help_width)) / 2;
        let help_y = area.y + (area.height.saturating_sub(help_height)) / 2;
        let help_area = Rect::new(help_x, help_y, help_width, help_height);
        let help_popup = Paragraph::new(help_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" help ")
                .title_style(
                    Style::default()
                        .fg(DRACULA_PURPLE)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(DRACULA_PURPLE))
                .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
        );
        frame.render_widget(Clear, help_area);
        frame.render_widget(help_popup, help_area);
    }
}

fn render_project_panel(frame: &mut Frame, _app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" project ")
        .title_style(
            Style::default()
                .fg(DRACULA_PURPLE)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(DRACULA_PURPLE))
        .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));

    let mut content = String::new();

    // Metrics first — glanceable status
    if let Some(metrics) = super::board::fetch_dora_metrics() {
        content.push_str(&metrics.format());
        content.push_str("\n\n---\n\n");
    }

    // Roadmap detail below
    match super::board::fetch_roadmap() {
        Ok(text) => content.push_str(&text),
        Err(_) => match super::board::fetch_board() {
            Ok(text) => content.push_str(&text),
            Err(e) => content.push_str(&format!("Project info unavailable: {e}")),
        },
    }

    let inner_width = area.width.saturating_sub(2) as usize;
    let prefix_spans = Vec::new();
    let indent = Span::raw("");
    let md_lines = render_md_message(&content, prefix_spans, indent.clone(), None, inner_width);
    let mut wrapped = Vec::new();
    for line in &md_lines {
        wrapped.extend(wrap_line_with_indent(line.clone(), &indent, inner_width));
    }

    let paragraph = Paragraph::new(wrapped).block(block).scroll((0, 0));
    frame.render_widget(paragraph, area);
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let full = frame.area();

    // Top-level split: content area + full-width status bar
    let v_split = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(full);
    let content_area = v_split[0];
    let status_area = v_split[1];

    let sidebar_width = if app.sidebar_open && full.width >= MIN_WIDTH_FOR_SIDEBAR {
        24u16
    } else {
        0
    };

    let h_chunks = Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(1)])
        .split(content_area);

    let main_area = h_chunks[1];

    let input_lines = if app.input.is_empty() {
        1
    } else {
        let width = main_area.width.saturating_sub(2) as usize;
        app.input.len().max(1).div_ceil(width.max(1))
    };
    let input_height = (input_lines + 2).min(8) as u16;

    let banner_height = broadcast_banner_height(app, main_area.width);

    let chunks = Layout::vertical([
        Constraint::Length(banner_height),
        Constraint::Min(3),
        Constraint::Length(input_height),
    ])
    .split(main_area);

    if sidebar_width > 0 {
        let s = h_chunks[0];
        app.sidebar_area = Some((s.x, s.y, s.width, s.height));
        render_sidebar(frame, app, h_chunks[0]);
    } else {
        app.sidebar_area = None;
    }
    app.status_bar_area = Some((
        status_area.x,
        status_area.y,
        status_area.width,
        status_area.height,
    ));

    render_broadcast_banner(frame, app, chunks[0]);

    if app.show_project_view && chunks[1].width >= 60 {
        // Side-by-side: project view (60%) | chat (40%)
        let split = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);
        app.chat_area = Some((split[0].x, split[0].y, split[0].width, split[0].height));
        render_project_panel(frame, app, split[0]);
        render_chat(frame, app, split[1]);
        render_unread_indicator(frame, app, split[1]);
    } else {
        let chat_rect = chunks[1];
        app.chat_area = Some((chat_rect.x, chat_rect.y, chat_rect.width, chat_rect.height));
        render_chat(frame, app, chunks[1]);
        render_unread_indicator(frame, app, chunks[1]);
    }
    render_input(frame, app, chunks[2]);
    render_popup(frame, app, chunks[2]);
    render_status_bar(frame, app, status_area);
    render_help_overlay(frame, app, main_area);
}
