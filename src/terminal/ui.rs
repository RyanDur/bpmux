use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::session;
use crate::terminal::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(3),    // chat pane
        Constraint::Length(3), // input pane
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    // Chat pane — only show chat messages
    let chat_messages: Vec<&session::Message> =
        app.messages.iter().filter(|m| m.is_chat()).collect();
    let chat_inner_width_for_truncation = frame.area().width.saturating_sub(4) as usize;
    let messages: Vec<Line> = chat_messages
        .iter()
        .enumerate()
        .flat_map(|(i, msg)| {
            // Short name: "ryan:human" → "ryan", "claude-code:ws" → "claude-code"
            let short_name = msg.from.split(':').next().unwrap_or(&msg.from);

            // Color by sender type: human = cyan, AI = green
            let is_human = msg.from.ends_with(":human");
            let name_color = if is_human { Color::Cyan } else { Color::Green };

            // Build the prefix (timestamp + name)
            let mut prefix = String::new();
            if app.show_timestamps {
                let time = msg.ts.format("%H:%M:%S").to_string();
                prefix.push_str(&format!("[{time}] "));
            }
            prefix.push_str(&format!("{short_name}: "));

            // Estimate wrapped line count for truncation decision
            let full_len = prefix.len() + msg.text.len();
            let wrapped_lines = if chat_inner_width_for_truncation > 0 {
                full_len.max(1).div_ceil(chat_inner_width_for_truncation)
            } else {
                1
            };

            let mut lines = Vec::new();

            if wrapped_lines > 5 {
                // Truncate: show first 3 lines worth of text, then [+N lines]
                let first_line_text_budget =
                    chat_inner_width_for_truncation.saturating_sub(prefix.len());
                let subsequent_budget = chat_inner_width_for_truncation * 2;
                let truncated_chars = first_line_text_budget + subsequent_budget;
                let display_text: String = msg.text.chars().take(truncated_chars).collect();
                let remaining_lines = wrapped_lines.saturating_sub(3);

                let mut spans = Vec::new();
                if app.show_timestamps {
                    let time = msg.ts.format("%H:%M:%S").to_string();
                    spans.push(Span::styled(
                        format!("[{time}] "),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                spans.push(Span::styled(
                    format!("{short_name}: "),
                    Style::default().fg(name_color).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(display_text));
                lines.push(Line::from(spans));

                lines.push(Line::from(Span::styled(
                    format!("  [+{remaining_lines} lines]"),
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                let mut spans = Vec::new();
                if app.show_timestamps {
                    let time = msg.ts.format("%H:%M:%S").to_string();
                    spans.push(Span::styled(
                        format!("[{time}] "),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                spans.push(Span::styled(
                    format!("{short_name}: "),
                    Style::default().fg(name_color).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(&msg.text));
                lines.push(Line::from(spans));
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

    // Auto-scroll: calculate total rendered lines including wrapping
    let chat_inner_width = chunks[0].width.saturating_sub(2) as usize; // minus borders
    let chat_height = chunks[0].height.saturating_sub(2) as usize;
    let total_lines: usize = messages
        .iter()
        .map(|line| {
            let line_width: usize = line.spans.iter().map(|s| s.content.len()).sum();
            if chat_inner_width > 0 {
                line_width.max(1).div_ceil(chat_inner_width)
            } else {
                1
            }
        })
        .sum();
    let max_scroll = if total_lines > chat_height {
        (total_lines - chat_height) as u16
    } else {
        0
    };
    let scroll = max_scroll.saturating_sub(app.scroll_offset);

    let chat = Paragraph::new(messages)
        .block(Block::default().borders(Borders::ALL).title(" bpmux "))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(chat, chunks[0]);

    // Input pane
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" > "))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[1]);

    // Show cursor at end of input text, accounting for wrapping
    #[allow(clippy::cast_possible_truncation)]
    let input_inner_width = chunks[1].width.saturating_sub(2) as usize;
    let (cursor_x, cursor_y) = if input_inner_width > 0 {
        let x = (app.input.len() % input_inner_width) as u16;
        let y = (app.input.len() / input_inner_width) as u16;
        (chunks[1].x + 1 + x, chunks[1].y + 1 + y)
    } else {
        (chunks[1].x + 1, chunks[1].y + 1)
    };
    frame.set_cursor_position((cursor_x, cursor_y));

    // Status bar
    let connection_status = if app.connected_peers == 0 {
        Span::styled(" solo", Style::default().fg(Color::Yellow))
    } else {
        Span::styled(
            format!(" {} connected", app.connected_peers),
            Style::default().fg(Color::Green),
        )
    };
    let mut status_spans = vec![
        connection_status,
        Span::raw(" | "),
        Span::raw(&app.session_id),
    ];
    if app.scroll_offset > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("scrolled up {}", app.scroll_offset),
            Style::default().fg(Color::DarkGray),
        ));
    }
    let status = Paragraph::new(Line::from(status_spans));
    frame.render_widget(status, chunks[2]);
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
            }],
        );

        terminal.draw(|frame| render(frame, &app)).unwrap();

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
            })
            .collect();

        // At scroll_offset 0 (bottom), last message visible
        let mut app = App::new("dev".to_string(), "test-id".to_string(), messages.clone());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let at_bottom = buffer_to_string(terminal.backend().buffer());
        assert!(
            at_bottom.contains("message 19"),
            "Bottom should show last message. Screen:\n{at_bottom}"
        );

        // Scroll up — earlier messages become visible, latest may not be
        app.scroll_up(15);
        terminal.draw(|frame| render(frame, &app)).unwrap();
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
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let at_bottom = buffer_to_string(terminal.backend().buffer());
        assert!(
            !at_bottom.contains("scrolled up"),
            "No scroll indicator at bottom"
        );

        app.scroll_up(3);
        terminal.draw(|frame| render(frame, &app)).unwrap();
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
        let app = App::new("dev".to_string(), "test-id".to_string(), vec![]);

        terminal.draw(|frame| render(frame, &app)).unwrap();
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

        terminal.draw(|frame| render(frame, &app)).unwrap();
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
        let app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![
                Message {
                    from: "dev:human".to_string(),
                    text: "visible chat".to_string(),
                    ts: Utc::now(),
                    message_type: None,
                },
                Message {
                    from: "ryan:claude-code".to_string(),
                    text: "[turn complete]".to_string(),
                    ts: Utc::now(),
                    message_type: Some("system".to_string()),
                },
            ],
        );

        terminal.draw(|frame| render(frame, &app)).unwrap();
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

        // A message long enough to exceed 5 wrapped lines at 78 chars inner width
        // 78 chars per line * 6 lines = 468 chars minimum
        let long_text = "word ".repeat(100); // ~500 chars, well over 5 lines

        let app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![Message {
                from: "bot:ws".to_string(),
                text: long_text,
                ts: Utc::now(),
                message_type: None,
            }],
        );

        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());

        assert!(
            screen.contains("+"),
            "Long message should show [+N lines] indicator. Screen:\n{screen}"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn short_messages_not_truncated() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = App::new(
            "dev".to_string(),
            "test-id".to_string(),
            vec![Message {
                from: "bot:ws".to_string(),
                text: "short message".to_string(),
                ts: Utc::now(),
                message_type: None,
            }],
        );

        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = buffer_to_string(terminal.backend().buffer());

        assert!(
            screen.contains("short message"),
            "Short message should render fully. Screen:\n{screen}"
        );
        assert!(
            !screen.contains("+"),
            "Short message should not be truncated. Screen:\n{screen}"
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
