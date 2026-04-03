use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::terminal::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(3),    // chat pane
        Constraint::Length(3), // input pane
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    // Chat pane
    let messages: Vec<Line> = app
        .messages
        .iter()
        .map(|msg| {
            let time = msg.ts.format("%H:%M:%S").to_string();
            Line::from(vec![
                Span::styled(format!("[{time}] "), Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}: ", msg.from),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&msg.text),
            ])
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
    let scroll = if total_lines > chat_height {
        (total_lines - chat_height) as u16
    } else {
        0
    };

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
    let status = Paragraph::new(Line::from(vec![
        Span::styled(" solo", Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::raw(&app.session_id),
    ]));
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
            }],
        );

        terminal.draw(|frame| render(frame, &app)).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("hello"));
        assert!(content.contains("ryan:human"));
        assert!(content.contains("solo"));
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
