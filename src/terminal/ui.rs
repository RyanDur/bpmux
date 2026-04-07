use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::session;
use crate::terminal::app::{self, App};

const DRACULA_BG: Color = Color::Rgb(40, 42, 54);
const DRACULA_FG: Color = Color::Rgb(248, 248, 242);
const DRACULA_SELECTION: Color = Color::Rgb(68, 71, 90);
const DRACULA_COMMENT: Color = Color::Rgb(98, 114, 164);
const DRACULA_PURPLE: Color = Color::Rgb(189, 147, 249);
const DRACULA_CYAN: Color = Color::Rgb(139, 233, 253);
const DRACULA_ORANGE: Color = Color::Rgb(255, 184, 108);
const DRACULA_GREEN: Color = Color::Rgb(80, 250, 123);
const DRACULA_RED: Color = Color::Rgb(255, 85, 85);
const DRACULA_PINK: Color = Color::Rgb(255, 121, 198);
const DRACULA_YELLOW: Color = Color::Rgb(241, 250, 140);

/// Dracula-themed stylesheet for tui-markdown rendering.
/// Replaces default ANSI colors that have poor contrast on dark backgrounds.
#[derive(Clone, Debug)]
struct DraculaStyleSheet;

impl tui_markdown::StyleSheet for DraculaStyleSheet {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::new()
                .fg(DRACULA_BG)
                .bg(DRACULA_CYAN)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            2 => Style::new().fg(DRACULA_CYAN).add_modifier(Modifier::BOLD),
            3 => Style::new()
                .fg(DRACULA_CYAN)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            _ => Style::new().fg(DRACULA_CYAN).add_modifier(Modifier::ITALIC),
        }
    }

    fn code(&self) -> Style {
        Style::new().fg(DRACULA_FG).bg(DRACULA_SELECTION)
    }

    fn link(&self) -> Style {
        Style::new()
            .fg(DRACULA_PURPLE)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        Style::new().fg(DRACULA_GREEN)
    }

    fn heading_meta(&self) -> Style {
        Style::new().add_modifier(Modifier::DIM)
    }

    fn metadata_block(&self) -> Style {
        Style::new().fg(DRACULA_YELLOW) // Dracula yellow
    }
}

const SENDER_PALETTE: [Color; 6] = [
    DRACULA_CYAN,   // Dracula cyan
    DRACULA_GREEN,  // Dracula green
    DRACULA_ORANGE, // Dracula orange
    DRACULA_PURPLE, // Dracula purple
    DRACULA_PINK,   // Dracula pink
    DRACULA_YELLOW, // Dracula yellow
];

fn sender_color_index(name: &str) -> usize {
    let hash: usize = name.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    });
    hash % SENDER_PALETTE.len()
}

fn sender_color(name: &str) -> Color {
    SENDER_PALETTE[sender_color_index(name)]
}

/// Color for channel tags. Broadcast = subdued Dracula comment gray.
/// Directed channels = sender palette offset by 3 so channel and sender
/// colors don't collide for the same name.
fn channel_tag_color(tag: &str, is_broadcast: bool) -> Color {
    if is_broadcast {
        DRACULA_COMMENT // Dracula comment gray (#6272A4)
    } else {
        SENDER_PALETTE[(sender_color_index(tag) + 3) % SENDER_PALETTE.len()]
    }
}

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
                    Span::styled(ch.chars().take(16).collect::<String>(), name_style),
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
            .title(" channels ")
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

/// Strip heading hash prefixes (e.g. "## ", "### ") from the first span in a line.
/// tui-markdown keeps hashes in the output; we remove them for GitHub-style rendering.
fn strip_heading_hashes(spans: &mut Vec<Span<'_>>) {
    if let Some(first) = spans.first_mut() {
        let trimmed = first.content.trim_start_matches('#');
        if trimmed.len() < first.content.len() {
            // Had leading hashes — strip them plus the trailing space
            let stripped = trimmed.trim_start();
            if stripped.is_empty() {
                spans.remove(0);
            } else {
                *first = Span::styled(stripped.to_string(), first.style);
            }
        }
    }
}

/// Render markdown message text into `Line`s, prepending prefix spans on the first
/// line and an indent on subsequent lines.  When `line_limit` is `Some(n)`, only
/// the first `n` markdown lines are emitted.
fn render_md_message<'a>(
    text: &'a str,
    prefix_spans: Vec<Span<'a>>,
    indent: Span<'a>,
    line_limit: Option<usize>,
) -> Vec<Line<'a>> {
    let options = tui_markdown::Options::new(DraculaStyleSheet);
    let md_text = tui_markdown::from_str_with_options(text, &options);
    let iter: Box<dyn Iterator<Item = (usize, Line<'_>)>> = match line_limit {
        Some(n) => Box::new(md_text.lines.into_iter().take(n).enumerate()),
        None => Box::new(md_text.lines.into_iter().enumerate()),
    };
    let mut lines = Vec::new();
    for (li, mut md_line) in iter {
        let mut spans = Vec::new();
        if li == 0 {
            spans.extend(prefix_spans.iter().cloned());
        } else {
            spans.push(indent.clone());
        }
        // Patch line-level style into each span so it survives Line reconstruction
        let line_style = md_line.style;
        for span in &mut md_line.spans {
            span.style = line_style.patch(span.style);
            // Replace hardcoded ANSI blues from tui-markdown internals
            if span.style.fg == Some(Color::LightBlue) {
                span.style.fg = Some(DRACULA_CYAN);
            } else if span.style.fg == Some(Color::Blue) {
                span.style.fg = Some(DRACULA_PURPLE);
            }
        }
        // Strip heading hash prefixes (e.g. "## ") — render as styled text only
        strip_heading_hashes(&mut md_line.spans);
        spans.extend(md_line.spans);
        lines.push(Line::from(spans));
    }
    lines
}

fn render_chat(frame: &mut Frame, app: &mut App, area: Rect) {
    let chat_messages: Vec<&session::Message> = app
        .messages
        .iter()
        .filter(|m| {
            if !m.is_chat() {
                return app.show_system_messages;
            }
            // Local system messages (from "bpmux") always pass the view filter
            if m.from == "bpmux" {
                return true;
            }
            match &app.current_view {
                None => true,
                Some(view) => app::channel_name(m, &app.peer_name) == *view,
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

    let messages: Vec<Line> = chat_messages
        .iter()
        .enumerate()
        .skip(start_idx)
        .flat_map(|(i, msg)| {
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

            let mut lines = Vec::new();

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
            prefix_spans.push(Span::styled(format!("{short_name}: "), name_style));

            if wrapped_lines > 10 && !app.expanded_messages.contains(&msg_key) {
                // Truncate: show first 7 lines of markdown, then [+N lines]
                let remaining_lines = wrapped_lines.saturating_sub(7);
                lines.extend(render_md_message(&msg.text, prefix_spans, indent, Some(7)));
                lines.push(Line::from(Span::styled(
                    format!("  [+{remaining_lines} lines]"),
                    Style::default().fg(DRACULA_COMMENT),
                )));
            } else {
                // Render full message text as markdown
                lines.extend(render_md_message(&msg.text, prefix_spans, indent, None));
            }

            // Add blank line separator only when the next message is from a different sender
            let next_is_different = chat_messages
                .get(i + 1)
                .is_some_and(|next| next.from != msg.from);

            if next_is_different {
                lines.push(Line::default());
            }

            lines
        })
        .collect();

    let chat = Paragraph::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" bpmux ")
                .title_style(Style::default().fg(DRACULA_PURPLE))
                .border_style(Style::default().fg(DRACULA_SELECTION))
                .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
        )
        .wrap(Wrap { trim: false });

    // Use ratatui's actual line count instead of estimating
    let total_lines = chat.line_count(chat_inner_width as u16);
    let max_scroll = total_lines.saturating_sub(chat_height);
    app.max_scroll = max_scroll;
    #[allow(clippy::cast_possible_truncation)]
    let scroll = max_scroll
        .saturating_sub(app.scroll_offset)
        .min(u16::MAX as usize) as u16;

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

fn render_pins(frame: &mut Frame, app: &App, area: Rect) {
    if !app.pins.is_empty() {
        let pin_lines: Vec<Line> = app
            .pins
            .iter()
            .enumerate()
            .map(|(i, p)| {
                Line::from(Span::styled(
                    format!(" {}. {}", i + 1, p),
                    Style::default().fg(DRACULA_FG), // default foreground
                ))
            })
            .collect();
        let pin_bar = Paragraph::new(pin_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" pins ")
                .title_style(
                    Style::default()
                        .fg(DRACULA_YELLOW)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(DRACULA_YELLOW)),
        );
        frame.render_widget(pin_bar, area);
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
                .border_style(Style::default().fg(DRACULA_SELECTION))
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

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let connection_status = if app.connected_peers == 0 {
        Span::styled(" solo", Style::default().fg(DRACULA_ORANGE))
    } else {
        Span::styled(
            format!(" {} connected", app.connected_peers),
            Style::default().fg(DRACULA_GREEN),
        )
    };
    let is_broadcast_view = app.current_view.as_deref() == Some("broadcast");
    let (view_label, channel_label) = if is_broadcast_view {
        let side_convos = app
            .active_channels
            .iter()
            .filter(|ch| *ch != "broadcast")
            .count();
        let label = format!(
            " \u{00B7} {} side convo{} \u{00B7} /view all",
            side_convos,
            if side_convos == 1 { "" } else { "s" }
        );
        (" [broadcast]".to_string(), label)
    } else {
        let channel_count = app.active_channels.len();
        let label = match channel_count {
            0 => String::new(),
            1 => " \u{00B7} 1 channel".to_string(),
            n => format!(" \u{00B7} {n} channels"),
        };
        let view = match &app.current_view {
            None => " [all]".to_string(),
            Some(v) => format!(" [{v}]"),
        };
        (view, label)
    };
    let mut status_spans = vec![
        Span::styled(
            view_label,
            Style::default().fg(DRACULA_CYAN), // Dracula cyan
        ),
        Span::styled(
            channel_label,
            Style::default().fg(DRACULA_COMMENT), // Dracula comment
        ),
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
        let short_name = peer.split(':').next().unwrap_or(peer);
        let dot_color = match status.as_str() {
            "busy" => DRACULA_YELLOW, // Yellow — busy
            "help" => DRACULA_RED,    // Red — needs help
            _ => DRACULA_GREEN,       // Green — available/connected
        };
        status_spans.push(Span::raw(" "));
        status_spans.push(Span::styled("\u{25CF}", Style::default().fg(dot_color)));
        status_spans.push(Span::styled(
            format!(" {short_name}"),
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
    let used: usize = status_spans.iter().map(|s| s.content.len()).sum();
    let bar_width = area.width as usize;
    let hint = "? help";
    if used + hint.len() + 1 < bar_width {
        let pad = bar_width.saturating_sub(used + hint.len());
        status_spans.push(Span::raw(" ".repeat(pad)));
        status_spans.push(Span::styled(hint, Style::default().fg(DRACULA_PURPLE)));
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
  /pin /pins /unpin  manage pins (max 5)
  /board   project board   /who    participants
  /status  set status      /debug  system msgs
  /quit    exit

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
        let help_x = (area.width.saturating_sub(help_width)) / 2;
        let help_y = (area.height.saturating_sub(help_height)) / 2;
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

pub fn render(frame: &mut Frame, app: &mut App) {
    // Auto-suppress sidebar if terminal is too narrow
    let sidebar_width = if app.sidebar_open && frame.area().width >= 60 {
        24u16
    } else {
        0
    };

    // Horizontal split: sidebar | main content
    let h_chunks = Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(1)])
        .split(frame.area());

    let main_area = h_chunks[1];

    let input_lines = if app.input.is_empty() {
        1
    } else {
        let width = main_area.width.saturating_sub(2) as usize; // borders only
        app.input.len().max(1).div_ceil(width.max(1))
    };
    let input_height = (input_lines + 2).min(8) as u16; // +2 for borders, cap at 8
    let pin_height = if app.pins.is_empty() {
        0
    } else {
        (app.pins.len() as u16) + 2 // +2 for borders
    };

    let chunks = Layout::vertical([
        Constraint::Min(3),               // chat pane
        Constraint::Length(pin_height),   // pin bar (0 if no pins)
        Constraint::Length(input_height), // input pane — grows with content
        Constraint::Length(1),            // status bar
    ])
    .split(main_area);

    if sidebar_width > 0 {
        render_sidebar(frame, app, h_chunks[0]);
    }

    render_chat(frame, app, chunks[0]);
    render_unread_indicator(frame, app, chunks[0]);
    render_pins(frame, app, chunks[1]);
    render_input(frame, app, chunks[2]);
    render_popup(frame, app, chunks[2]);
    render_status_bar(frame, app, chunks[3]);
    render_help_overlay(frame, app, frame.area());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Message;
    use chrono::Utc;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn test_app_with(messages: Vec<Message>) -> App {
        App::new("dev".to_string(), "test-id".to_string(), messages)
    }

    fn chat_msg(from: &str, text: &str) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
        }
    }

    fn system_msg(from: &str, text: &str) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: Some("system".to_string()),
            to: None,
        }
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(
            "ryan".to_string(),
            "test-id".to_string(),
            vec![chat_msg("ryan:human", "hello")],
        );

        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("hello"));
        assert!(content.contains("ryan:"));
        assert!(content.contains("solo"));
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn scroll_offset_shows_earlier_messages() {
        let backend = TestBackend::new(80, 15); // enough for chat + input + status
        let mut terminal = Terminal::new(backend).unwrap();

        // Create enough messages to overflow the chat pane
        let messages: Vec<Message> = (0..20)
            .map(|i| chat_msg("dev:human", &format!("message {i}")))
            .collect();

        // At scroll_offset 0 (bottom), last message visible
        let mut app = test_app_with(messages.clone());
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let at_bottom = buffer_to_string(terminal.backend().buffer());
        assert!(
            at_bottom.contains("message 19"),
            "Bottom should show last message. Screen:\n{at_bottom}"
        );

        // Scroll up — earlier messages become visible, latest may not be
        app.scroll_up(15);
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let scrolled = buffer_to_string(terminal.backend().buffer());
        assert!(
            scrolled.contains("message 0"),
            "Scrolled up should show earlier messages. Screen:\n{scrolled}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn scroll_indicator_shows_when_scrolled_up() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = test_app_with(vec![]);
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let at_bottom = buffer_to_string(terminal.backend().buffer());
        assert!(
            !at_bottom.contains("scrolled up"),
            "No scroll indicator at bottom"
        );

        app.max_scroll = 100;
        app.scroll_up(3);
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let scrolled = buffer_to_string(terminal.backend().buffer());
        assert!(
            scrolled.contains("scrolled up 3"),
            "Should show scroll indicator. Screen:\n{scrolled}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn status_bar_shows_solo_when_no_peers() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = test_app_with(vec![]);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());
        assert!(
            screen.contains("solo"),
            "Should show 'solo' with no peers. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn status_bar_shows_peer_count_when_connected() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = test_app_with(vec![]);
        app.connected_peers = 2;

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());
        assert!(
            screen.contains("2 connected"),
            "Should show '2 connected' with peers. Screen:\n{screen}"
        );
        assert!(
            !screen.contains("solo"),
            "Should not show 'solo' when peers connected. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn system_messages_filtered_from_chat() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = test_app_with(vec![
            chat_msg("dev:human", "visible chat"),
            system_msg("ryan:claude-code", "[turn complete]"),
        ]);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());
        assert!(
            screen.contains("visible chat"),
            "Chat messages should appear. Screen:\n{screen}"
        );
        assert!(
            !screen.contains("turn complete"),
            "System messages should be filtered. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn long_messages_truncated_with_indicator() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // 78 chars inner width. 11+ wrapped lines should truncate.
        // "bot: " prefix = 5 chars, so 73 chars of text per first line, 78 per subsequent
        // 73 + 78*10 = 853 chars = 11 lines.
        let long_text = "word ".repeat(200); // ~1000 chars, well over 10 lines

        let mut app = test_app_with(vec![chat_msg("bot:ws", &long_text)]);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());

        assert!(
            screen.contains("[+"),
            "Long message should show [+N lines] indicator. Screen:\n{screen}"
        );
        assert!(
            screen.contains("lines]"),
            "Indicator should show line count. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn short_messages_not_truncated() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = test_app_with(vec![chat_msg("bot:ws", "short message")]);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());

        assert!(
            screen.contains("short message"),
            "Short message should render fully. Screen:\n{screen}"
        );
        assert!(
            !screen.contains("[+"),
            "Short message should not be truncated. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn ten_line_message_not_truncated() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // 78 inner width. "[broadcast] bot: " = 17 chars prefix.
        // First line fits 61 chars of text. 10 lines = 61 + 78*9 = 763 chars.
        let text_for_ten_lines = "a".repeat(763);

        let mut app = test_app_with(vec![chat_msg("bot:ws", &text_for_ten_lines)]);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());

        assert!(
            !screen.contains("[+"),
            "10-line message should NOT be truncated. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn eleven_line_message_is_truncated() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // 11 lines = 61 + 78*10 = 841 chars
        let text_for_eleven_lines = "a".repeat(841);

        let mut app = test_app_with(vec![chat_msg("bot:ws", &text_for_eleven_lines)]);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());

        assert!(
            screen.contains("[+"),
            "11-line message SHOULD be truncated. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn status_bar_shows_unread_count_when_scrolled_up() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = test_app_with(vec![]);
        app.unread_count = 3;

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());
        assert!(
            screen.contains("3 new"),
            "Should show '3 new' unread indicator. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn renders_without_panic_with_long_input() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new("ryan".to_string(), "test-id".to_string(), vec![]);
        app.input = "a".repeat(500);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        let screen = buffer_to_string(terminal.backend().buffer());
        assert!(
            screen.contains("ryan >"),
            "Input title should include peer name. Screen:\n{screen}"
        );
    }

    #[test]
    fn team_names_get_distinct_expected_colors() {
        // Pin exact colors for our team names — catches hash changes that
        // would cause collisions between ryan, dev, and pm
        let ryan_color = super::sender_color("ryan");
        let dev_color = super::sender_color("dev");
        let pm_color = super::sender_color("pm");

        assert_ne!(ryan_color, dev_color, "ryan and dev must differ");
        assert_ne!(ryan_color, pm_color, "ryan and pm must differ");
        assert_ne!(dev_color, pm_color, "dev and pm must differ");
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn system_messages_visible_when_debug_on() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = test_app_with(vec![
            chat_msg("dev:human", "visible chat"),
            system_msg("ryan:claude-code", "[turn complete]"),
        ]);
        app.show_system_messages = true;

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());
        assert!(
            screen.contains("visible chat"),
            "Chat messages should appear. Screen:\n{screen}"
        );
        assert!(
            screen.contains("turn complete"),
            "System messages should appear when debug is on. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn message_with_code_block_renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = test_app_with(vec![chat_msg(
            "dev:human",
            "before\n```\nlet x = 1;\n```\nafter",
        )]);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());
        assert!(
            screen.contains("before"),
            "Text before code block should render. Screen:\n{screen}"
        );
        assert!(
            screen.contains("let x = 1;"),
            "Code block content should render. Screen:\n{screen}"
        );
        assert!(
            screen.contains("after"),
            "Text after code block should render. Screen:\n{screen}"
        );
        // tui-markdown handles code block rendering — fences may or may not appear
        // depending on markdown parsing context (blank lines before fences etc.)
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn markdown_heading_styles_survive_line_reconstruction() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = test_app_with(vec![chat_msg("bpmux:ws", "## Board\n- item one")]);

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        // Find the "Board" text in the buffer and check its style
        let mut found_board = false;
        for y in 0..buffer.area.height {
            let mut row = String::new();
            for x in 0..buffer.area.width {
                if let Some(cell) = buffer.cell((x, y)) {
                    row.push(cell.symbol().chars().next().unwrap_or(' '));
                }
            }
            if row.contains("Board") {
                // Find the exact column of 'B' in "Board"
                if let Some(col) = row.find('B') {
                    let cell = buffer.cell((col as u16, y)).unwrap();
                    // Heading 2 should be cyan + bold
                    assert_eq!(
                        cell.fg, DRACULA_CYAN,
                        "H2 heading should be Dracula cyan. Row: {row}"
                    );
                    assert!(
                        cell.modifier.contains(Modifier::BOLD),
                        "H2 heading should be bold. Row: {row}"
                    );
                    found_board = true;
                }
                break;
            }
        }
        assert!(
            found_board,
            "Should find 'Board' heading in rendered output"
        );

        // Verify hashes are stripped — no "##" should appear in the rendered buffer
        let screen = buffer_to_string(buffer);
        assert!(
            !screen.contains("##"),
            "Heading hashes should be stripped. Screen:\n{screen}"
        );
    }

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut s = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                s.push(
                    buffer
                        .cell((x, y))
                        .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
                );
            }
            s.push('\n');
        }
        s
    }
}
