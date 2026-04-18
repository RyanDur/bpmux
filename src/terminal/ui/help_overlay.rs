use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::terminal::app::App;
use crate::terminal::commands::help::format_command_list;
use crate::terminal::theme::{DRACULA_BG, DRACULA_FG, DRACULA_PURPLE};

pub(super) const HELP_OVERLAY_WIDTH: u16 = 52;

pub(super) fn render_help_overlay(frame: &mut Frame, app: &App, area: Rect) {
    if matches!(app.input_mode, crate::terminal::app::InputMode::Help) {
        // Source the command list from `SLASH_COMMANDS` (#164) so the
        // overlay and the `/help` slash command never drift. Two-column
        // form to fit alongside the keys section in the 24-row terminal
        // budget — full descriptions live in the `/help` chat output.
        let mut help_text = format_command_list(2);
        // Compact the Keys + footer to keep the overlay within the
        // 22-line budget (24 rows minus 2 borders). The grouping
        // (#284) added ~5 lines of headers + an odd-row in Meta; we
        // claw that back by dropping the blank lines around Keys
        // and the footer. Keys still visually distinct via the
        // "Keys:" label itself.
        help_text.push_str(
            "\nKeys:\n  Enter    send / expand    Esc     clear input\n  \
Ctrl+U   scroll up        Ctrl+D  scroll down\n  \u{2325}1       channel sidebar   \
Ctrl+T  timestamps\n  Ctrl+C   quit (empty)     ?  this overlay\n\
click anywhere or press any key to close",
        );

        let help_lines: Vec<Line> = help_text
            .lines()
            .map(|l| Line::from(Span::raw(l.to_string())))
            .collect();
        let help_height = (help_lines.len() as u16 + 2).min(area.height); // +2 for borders
        let help_width = HELP_OVERLAY_WIDTH.min(area.width);
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
