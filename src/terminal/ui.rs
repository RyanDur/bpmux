use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::session;
use crate::terminal::app::{self, App};

const SENDER_PALETTE: [Color; 6] = [
    Color::Rgb(139, 233, 253), // Dracula cyan
    Color::Rgb(80, 250, 123),  // Dracula green
    Color::Rgb(255, 184, 108), // Dracula orange
    Color::Rgb(189, 147, 249), // Dracula purple
    Color::Rgb(255, 121, 198), // Dracula pink
    Color::Rgb(241, 250, 140), // Dracula yellow
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
        Color::Rgb(98, 114, 164) // Dracula comment gray (#6272A4)
    } else {
        SENDER_PALETTE[(sender_color_index(tag) + 3) % SENDER_PALETTE.len()]
    }
}

fn parse_inline_code(text: &str) -> Vec<(String, bool)> {
    let mut segments = Vec::new();
    let mut remaining = text;
    while let Some(open) = remaining.find('`') {
        if open > 0 {
            segments.push((remaining[..open].to_string(), false));
        }
        let after_open = &remaining[open + 1..];
        if let Some(close) = after_open.find('`') {
            segments.push((after_open[..close].to_string(), true));
            remaining = &after_open[close + 1..];
        } else {
            // No closing backtick — treat as literal
            segments.push((remaining[open..open + 1].to_string(), false));
            remaining = &remaining[open + 1..];
        }
    }
    if !remaining.is_empty() {
        segments.push((remaining.to_string(), false));
    }
    segments
}

fn parse_mentions(text: &str) -> Vec<Span<'static>> {
    let inline_segments = parse_inline_code(text);
    let mut spans = Vec::new();
    for (segment, is_code) in inline_segments {
        if is_code {
            spans.push(Span::styled(
                segment,
                Style::default().fg(Color::Rgb(255, 184, 108)),
            ));
        } else {
            let mut remaining = segment.as_str();
            while let Some(at_pos) = remaining.find('@') {
                if at_pos > 0 {
                    spans.push(Span::raw(remaining[..at_pos].to_string()));
                }
                let after_at = &remaining[at_pos + 1..];
                let mention_end = after_at
                    .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                    .unwrap_or(after_at.len());
                if mention_end > 0 {
                    let mention = &remaining[at_pos..at_pos + 1 + mention_end];
                    spans.push(Span::styled(
                        mention.to_string(),
                        Style::default()
                            .fg(Color::Rgb(139, 233, 253))
                            .add_modifier(Modifier::BOLD),
                    ));
                    remaining = &remaining[at_pos + 1 + mention_end..];
                } else {
                    spans.push(Span::raw(remaining[at_pos..at_pos + 1].to_string()));
                    remaining = &remaining[at_pos + 1..];
                }
            }
            if !remaining.is_empty() {
                spans.push(Span::raw(remaining.to_string()));
            }
        }
    }
    spans
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let input_lines = if app.input.is_empty() {
        1
    } else {
        let width = frame.area().width.saturating_sub(4) as usize; // borders + padding
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
    .split(frame.area());

    // Chat pane — show chat messages (and system messages when debug is on),
    // filtered by current_view when set.
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
                Some(view) => app::channel_name(m) == *view,
            }
        })
        .collect();
    let chat_inner_width = chunks[0].width.saturating_sub(2) as usize;
    let chat_height = chunks[0].height.saturating_sub(2) as usize;

    // Virtualization: only render messages near the visible window
    let visible_window = chat_height * 3; // generous buffer for scroll
    let total_msgs = chat_messages.len();
    let scroll_back = app.scroll_offset as usize;
    let needed = visible_window + scroll_back;
    let start_idx = total_msgs.saturating_sub(needed);

    let messages: Vec<Line> = chat_messages
        .iter()
        .enumerate()
        .skip(start_idx)
        .flat_map(|(i, msg)| {
            // Short name: "ryan:human" → "ryan", "claude-code:ws" → "claude-code"
            let short_name = msg.from.split(':').next().unwrap_or(&msg.from);

            // Deterministic color from sender name — same name always gets same color
            let is_human = msg.from.ends_with(":human");
            let name_color = sender_color(short_name);

            // Channel tag: [broadcast], [a↔b] (DM), [a+b+c] (group)
            let (channel_tag, is_broadcast) = match &msg.to {
                None => ("[broadcast] ".to_string(), true),
                Some(recipients) if recipients.iter().any(|r| r == "all") => {
                    ("[broadcast] ".to_string(), true)
                }
                Some(recipients) => {
                    let mut names: Vec<&str> = recipients.iter().map(|s| s.as_str()).collect();
                    let sender_short = msg.from.split(':').next().unwrap_or(&msg.from);
                    if !names.contains(&sender_short) {
                        names.push(sender_short);
                    }
                    names.sort();
                    let tag = if names.len() == 2 {
                        format!("[{}↔{}] ", names[0], names[1])
                    } else {
                        format!("[{}] ", names.join("+"))
                    };
                    (tag, false)
                }
            };

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

            let ts_style = Style::default().fg(Color::Rgb(98, 114, 164));
            let tag_color = channel_tag_color(&channel_tag, is_broadcast);
            let tag_style = Style::default().fg(tag_color);

            if wrapped_lines > 10 && !app.expanded_messages.contains(&i) {
                // Truncate: show first 7 lines worth of text, then [+N lines]
                let first_line_text_budget = chat_inner_width.saturating_sub(prefix.len());
                let subsequent_budget = chat_inner_width * 6;
                let truncated_chars = first_line_text_budget + subsequent_budget;
                let display_text: String = msg.text.chars().take(truncated_chars).collect();
                let remaining_lines = wrapped_lines.saturating_sub(7);

                let mut spans = Vec::new();
                if app.show_timestamps {
                    let time = msg.ts.format("%H:%M:%S").to_string();
                    spans.push(Span::styled(format!("[{time}] "), ts_style));
                }
                spans.push(Span::styled(channel_tag.clone(), tag_style));
                spans.push(Span::styled(
                    format!("{short_name}: "),
                    if is_human {
                        Style::default().fg(name_color).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(name_color)
                    },
                ));
                spans.extend(parse_mentions(&display_text));
                lines.push(Line::from(spans));

                lines.push(Line::from(Span::styled(
                    format!("  [+{remaining_lines} lines]"),
                    Style::default().fg(Color::Rgb(98, 114, 164)),
                )));
            } else {
                let code_style = Style::default()
                    .fg(Color::Rgb(248, 248, 242))
                    .bg(Color::Rgb(68, 71, 90));
                let text_lines: Vec<&str> = msg.text.split('\n').collect();
                let mut in_code_block = false;
                for (li, text_line) in text_lines.iter().enumerate() {
                    if text_line.trim() == "```"
                        || text_line.trim().starts_with("```") && !in_code_block
                    {
                        in_code_block = !in_code_block;
                        continue;
                    }
                    if in_code_block && text_line.trim().ends_with("```") {
                        // Closing fence on same line as content — render content, then exit
                        let content = text_line.trim().trim_end_matches("```");
                        let mut spans = Vec::new();
                        if li == 0 {
                            if app.show_timestamps {
                                let time = msg.ts.format("%H:%M:%S").to_string();
                                spans.push(Span::styled(format!("[{time}] "), ts_style));
                            }
                            spans.push(Span::styled(channel_tag.clone(), tag_style));
                            spans.push(Span::styled(
                                format!("{short_name}: "),
                                if is_human {
                                    Style::default().fg(name_color).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(name_color)
                                },
                            ));
                        }
                        if !content.is_empty() {
                            spans.push(Span::styled(format!("  {content}"), code_style));
                        }
                        if !spans.is_empty() {
                            lines.push(Line::from(spans));
                        }
                        in_code_block = false;
                        continue;
                    }
                    let mut spans = Vec::new();
                    if li == 0 {
                        if app.show_timestamps {
                            let time = msg.ts.format("%H:%M:%S").to_string();
                            spans.push(Span::styled(format!("[{time}] "), ts_style));
                        }
                        spans.push(Span::styled(channel_tag.clone(), tag_style));
                        spans.push(Span::styled(
                            format!("{short_name}: "),
                            if is_human {
                                Style::default().fg(name_color).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(name_color)
                            },
                        ));
                    } else if !in_code_block {
                        // Indent continuation to align with body start
                        spans.push(Span::raw(" ".repeat(prefix.len())));
                    }
                    if in_code_block {
                        spans.push(Span::styled(format!("  {text_line}"), code_style));
                    } else {
                        spans.extend(parse_mentions(text_line));
                    }
                    lines.push(Line::from(spans));
                }
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
                .title_style(Style::default().fg(Color::Rgb(189, 147, 249)))
                .border_style(Style::default().fg(Color::Rgb(68, 71, 90)))
                .style(
                    Style::default()
                        .bg(Color::Rgb(40, 42, 54))
                        .fg(Color::Rgb(248, 248, 242)),
                ),
        )
        .wrap(Wrap { trim: false });

    // Use ratatui's actual line count instead of estimating
    let total_lines = chat.line_count(chat_inner_width as u16);
    let max_scroll = if total_lines > chat_height {
        (total_lines - chat_height) as u16
    } else {
        0
    };
    app.max_scroll = max_scroll;
    let scroll = max_scroll.saturating_sub(app.scroll_offset);

    let chat = chat.scroll((scroll, 0));
    frame.render_widget(chat, chunks[0]);

    // Pin bar — rendered between chat and input when pins exist
    if !app.pins.is_empty() {
        let pin_lines: Vec<Line> = app
            .pins
            .iter()
            .enumerate()
            .map(|(i, p)| {
                Line::from(Span::styled(
                    format!(" {}. {}", i + 1, p),
                    Style::default().fg(Color::Rgb(241, 250, 140)), // Dracula yellow text
                ))
            })
            .collect();
        let pin_bar = Paragraph::new(pin_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" pins ")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(241, 250, 140))
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Color::Rgb(241, 250, 140)))
                .style(
                    Style::default()
                        .bg(Color::Rgb(40, 42, 54)) // Dracula background
                        .fg(Color::Rgb(241, 250, 140)),
                ),
        );
        frame.render_widget(pin_bar, chunks[1]);
    }

    // Input pane — with syntax coloring for slash commands
    let input_line = if app.input.starts_with('/') {
        // Color the command keyword differently from the arguments
        let cmd_end = app.input.find(' ').unwrap_or(app.input.len());
        let cmd = &app.input[..cmd_end];
        let rest = &app.input[cmd_end..];
        Line::from(vec![
            Span::styled(
                cmd.to_string(),
                Style::default().fg(Color::Rgb(189, 147, 249)), // Dracula purple
            ),
            Span::styled(
                rest.to_string(),
                Style::default().fg(Color::Rgb(248, 248, 242)), // Default fg
            ),
        ])
    } else {
        Line::from(app.input.as_str())
    };
    let input = Paragraph::new(input_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} > ", app.peer_name))
                .title_style(Style::default().fg(Color::Rgb(189, 147, 249)))
                .border_style(Style::default().fg(Color::Rgb(68, 71, 90)))
                .style(
                    Style::default()
                        .bg(Color::Rgb(40, 42, 54))
                        .fg(Color::Rgb(248, 248, 242)),
                ),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[2]);

    // Show cursor at end of input text, accounting for wrapping
    #[allow(clippy::cast_possible_truncation)]
    let input_inner_width = chunks[2].width.saturating_sub(2) as usize;
    let (cursor_x, cursor_y) = if input_inner_width > 0 {
        let x = (app.cursor_pos % input_inner_width) as u16;
        let y = (app.cursor_pos / input_inner_width) as u16;
        (chunks[2].x + 1 + x, chunks[2].y + 1 + y)
    } else {
        (chunks[2].x + 1, chunks[2].y + 1)
    };
    frame.set_cursor_position((cursor_x, cursor_y));

    // Command autocomplete popup — shown when input starts with `/`
    let matches = app.popup_matches();
    if !matches.is_empty() {
        let popup_height = matches.len() as u16 + 2; // +2 for borders
        let popup_width = 40u16.min(chunks[2].width);
        // Position popup above the input pane
        let popup_y = chunks[2].y.saturating_sub(popup_height);
        let popup_area = Rect::new(chunks[2].x, popup_y, popup_width, popup_height);

        let items: Vec<Line> = matches
            .iter()
            .enumerate()
            .map(|(i, (cmd, desc))| {
                let selected = app.popup_selected == Some(i);
                let (cmd_style, desc_style) = if selected {
                    (
                        Style::default()
                            .fg(Color::Rgb(248, 248, 242))
                            .bg(Color::Rgb(68, 71, 90)),
                        Style::default()
                            .fg(Color::Rgb(189, 147, 249))
                            .bg(Color::Rgb(68, 71, 90)),
                    )
                } else {
                    (
                        Style::default().fg(Color::Rgb(189, 147, 249)),
                        Style::default().fg(Color::Rgb(98, 114, 164)),
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
                .border_style(Style::default().fg(Color::Rgb(98, 114, 164)))
                .style(
                    Style::default()
                        .bg(Color::Rgb(40, 42, 54))
                        .fg(Color::Rgb(248, 248, 242)),
                ),
        );
        frame.render_widget(Clear, popup_area);
        frame.render_widget(popup, popup_area);
    }

    // Status bar
    let connection_status = if app.connected_peers == 0 {
        Span::styled(" solo", Style::default().fg(Color::Rgb(255, 184, 108)))
    } else {
        Span::styled(
            format!(" {} connected", app.connected_peers),
            Style::default().fg(Color::Rgb(80, 250, 123)),
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
            Style::default().fg(Color::Rgb(139, 233, 253)), // Dracula cyan
        ),
        Span::styled(
            channel_label,
            Style::default().fg(Color::Rgb(98, 114, 164)), // Dracula comment
        ),
        Span::raw(" "),
        connection_status,
    ];
    if app.scroll_offset > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("scrolled up {}", app.scroll_offset),
            Style::default().fg(Color::Rgb(98, 114, 164)),
        ));
    }
    if app.unread_count > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("{} new", app.unread_count),
            Style::default().fg(Color::Rgb(255, 121, 198)),
        ));
    }
    let mut peers: Vec<_> = app.peer_activity.iter().collect();
    peers.sort_by_key(|(name, _)| (*name).clone());
    for (peer, (status, _)) in &peers {
        let short_name = peer.split(':').next().unwrap_or(peer);
        let dot_color = match status.as_str() {
            "busy" => Color::Rgb(241, 250, 140), // Yellow — busy
            "help" => Color::Rgb(255, 85, 85),   // Red — needs help
            _ => Color::Rgb(80, 250, 123),       // Green — available/connected
        };
        status_spans.push(Span::raw(" "));
        status_spans.push(Span::styled("\u{25CF}", Style::default().fg(dot_color)));
        status_spans.push(Span::styled(
            format!(" {short_name}"),
            Style::default().fg(Color::Rgb(248, 248, 242)),
        ));
    }
    // DM unread badge with sender names
    if !app.dm_unread_from.is_empty() {
        let names = app.dm_unread_from.join(", ");
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("\u{2709} DM from {names}"),
            Style::default().fg(Color::Rgb(241, 250, 140)), // Dracula yellow
        ));
    }
    let status = Paragraph::new(Line::from(status_spans)).style(
        Style::default()
            .bg(Color::Rgb(68, 71, 90))
            .fg(Color::Rgb(248, 248, 242)),
    );
    frame.render_widget(status, chunks[3]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Message;
    use chrono::Utc;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(
            "ryan".to_string(),
            "test-id".to_string(),
            vec![Message {
                from: "ryan:human".to_string(),
                text: "hello".to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            }],
        );

        let mut app = app;
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
            .map(|i| Message {
                from: "dev:human".to_string(),
                text: format!("message {i}"),
                ts: Utc::now(),
                message_type: None,
                to: None,
            })
            .collect();

        // At scroll_offset 0 (bottom), last message visible
        let mut app = App::new("dev".to_string(), "test-id".to_string(), messages.clone());
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

        let mut app = App::new("dev".to_string(), "test-id".to_string(), vec![]);
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
        let mut app = App::new("dev".to_string(), "test-id".to_string(), vec![]);

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
        let mut app = App::new("dev".to_string(), "test-id".to_string(), vec![]);
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
        let mut app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![
                Message {
                    from: "dev:human".to_string(),
                    text: "visible chat".to_string(),
                    ts: Utc::now(),
                    message_type: None,
                    to: None,
                },
                Message {
                    from: "ryan:claude-code".to_string(),
                    text: "[turn complete]".to_string(),
                    ts: Utc::now(),
                    message_type: Some("system".to_string()),
                    to: None,
                },
            ],
        );

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

        let mut app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![Message {
                from: "bot:ws".to_string(),
                text: long_text,
                ts: Utc::now(),
                message_type: None,
                to: None,
            }],
        );

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

        let mut app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![Message {
                from: "bot:ws".to_string(),
                text: "short message".to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            }],
        );

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

        let mut app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![Message {
                from: "bot:ws".to_string(),
                text: text_for_ten_lines,
                ts: Utc::now(),
                message_type: None,
                to: None,
            }],
        );

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

        let mut app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![Message {
                from: "bot:ws".to_string(),
                text: text_for_eleven_lines,
                ts: Utc::now(),
                message_type: None,
                to: None,
            }],
        );

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
        let mut app = App::new("dev".to_string(), "test-id".to_string(), vec![]);
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

    #[test]
    fn parse_mentions_highlights_mention_in_text() {
        let spans = super::parse_mentions("hello @dev how are you");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "hello ");
        assert_eq!(spans[1].content, "@dev");
        assert_eq!(
            spans[1].style,
            Style::default()
                .fg(Color::Rgb(139, 233, 253))
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(spans[2].content, " how are you");
    }

    #[test]
    fn parse_mentions_no_mentions_returns_single_span() {
        let spans = super::parse_mentions("no mentions");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "no mentions");
    }

    #[test]
    fn parse_mentions_multiple_mentions() {
        let spans = super::parse_mentions("@dev @pm");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "@dev");
        assert_eq!(
            spans[0].style,
            Style::default()
                .fg(Color::Rgb(139, 233, 253))
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(spans[1].content, " ");
        assert_eq!(spans[2].content, "@pm");
        assert_eq!(
            spans[2].style,
            Style::default()
                .fg(Color::Rgb(139, 233, 253))
                .add_modifier(Modifier::BOLD)
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn system_messages_visible_when_debug_on() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![
                Message {
                    from: "dev:human".to_string(),
                    text: "visible chat".to_string(),
                    ts: Utc::now(),
                    message_type: None,
                    to: None,
                },
                Message {
                    from: "ryan:claude-code".to_string(),
                    text: "[turn complete]".to_string(),
                    ts: Utc::now(),
                    message_type: Some("system".to_string()),
                    to: None,
                },
            ],
        );
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
        let mut app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![Message {
                from: "dev:human".to_string(),
                text: "before\n```\nlet x = 1;\n```\nafter".to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            }],
        );

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
        assert!(
            !screen.contains("```"),
            "Fence markers should not render. Screen:\n{screen}"
        );
    }

    #[test]
    fn parse_inline_code_detects_backtick_spans() {
        let segments = super::parse_inline_code("hello `code` world");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], ("hello ".to_string(), false));
        assert_eq!(segments[1], ("code".to_string(), true));
        assert_eq!(segments[2], (" world".to_string(), false));
    }

    #[test]
    fn parse_inline_code_no_backticks() {
        let segments = super::parse_inline_code("no code here");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0], ("no code here".to_string(), false));
    }

    #[test]
    fn parse_inline_code_unclosed_backtick_treated_as_literal() {
        let segments = super::parse_inline_code("hello `unclosed");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], ("hello ".to_string(), false));
        assert_eq!(segments[1], ("`".to_string(), false));
        assert_eq!(segments[2], ("unclosed".to_string(), false));
    }

    #[test]
    fn parse_mentions_styles_inline_code_with_orange() {
        let spans = super::parse_mentions("use `cargo build` to compile");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "use ");
        assert_eq!(spans[1].content, "cargo build");
        assert_eq!(
            spans[1].style,
            Style::default().fg(Color::Rgb(255, 184, 108))
        );
        assert_eq!(spans[2].content, " to compile");
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
