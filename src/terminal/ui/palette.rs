use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::terminal::app::App;
use crate::terminal::icons;
use crate::terminal::theme::{
    DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_PURPLE, DRACULA_SELECTION,
};

const PALETTE_WIDTH: u16 = 24;

pub(super) fn render_command_palette(frame: &mut Frame, app: &App, area: Rect) {
    if !matches!(app.input_mode, crate::terminal::app::InputMode::Palette) {
        return;
    }
    let matches = app.palette_matches();
    let palette_height = (matches.len() as u16 + 4).min(area.height); // +4 for border + input + gap
    let palette_width = PALETTE_WIDTH.min(area.width);
    let palette_y = area.y + (area.height.saturating_sub(palette_height)) / 2;
    let palette_x = area.x;
    let palette_area = Rect::new(palette_x, palette_y, palette_width, palette_height);

    // Build the content lines.
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" > {}_", app.palette.input),
        Style::default().fg(DRACULA_FG),
    )));
    lines.push(Line::default());
    for (i, (name, short)) in matches.iter().enumerate() {
        let selected = i == app.palette.selected;
        let indicator = if selected {
            format!("{} ", icons::EXPAND)
        } else {
            "  ".to_string()
        };
        let style = if selected {
            Style::default().fg(DRACULA_CYAN)
        } else {
            Style::default().fg(DRACULA_FG)
        };
        lines.push(Line::from(vec![
            Span::styled(indicator, style),
            Span::styled(*name, style),
            Span::styled(format!("  {short}"), Style::default().fg(DRACULA_COMMENT)),
        ]));
    }

    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" actions ")
            .title_style(
                Style::default()
                    .fg(DRACULA_PURPLE)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(DRACULA_COMMENT))
            .style(Style::default().bg(DRACULA_SELECTION).fg(DRACULA_FG)),
    );
    frame.render_widget(Clear, palette_area);
    frame.render_widget(popup, palette_area);
}
