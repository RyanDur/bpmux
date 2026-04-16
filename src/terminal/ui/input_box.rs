use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders},
};

use crate::terminal::app::App;
use crate::terminal::theme::{DRACULA_BG, DRACULA_FG, DRACULA_PURPLE, DRACULA_SELECTION};

pub(super) fn render_input(frame: &mut Frame, app: &mut App, area: Rect) {
    // Show active topic in the input prompt so the user knows
    // which topic they're typing into.
    let topic_label = if app.topic.active == "general" {
        String::new()
    } else {
        format!(" ({})", app.topic.active)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(match &app.channels.send_target {
            Some(ch) => format!(" {} [{ch}]{topic_label} > ", app.identity.name),
            None => format!(" {}{topic_label} > ", app.identity.name),
        })
        .title_style(Style::default().fg(DRACULA_PURPLE))
        .border_style(Style::default().fg(if app.sidebar.focused {
            DRACULA_SELECTION
        } else {
            DRACULA_PURPLE
        }))
        .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));

    app.textarea.set_block(block);
    app.textarea
        .set_style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));
    app.textarea
        .set_cursor_line_style(Style::default().bg(DRACULA_BG));
    frame.render_widget(&app.textarea, area);
}
