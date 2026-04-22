use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders},
};

use crate::terminal::app::App;
use crate::terminal::theme::{DRACULA_BG, DRACULA_FG, DRACULA_PURPLE, DRACULA_SELECTION};

pub(super) fn render_input(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(match &app.channels.send_target {
            Some(ch) => format!(" {} [{ch}] > ", app.identity.name),
            None => format!(" {} > ", app.identity.name),
        })
        .title_style(Style::default().fg(DRACULA_PURPLE))
        .border_style(Style::default().fg(if app.sidebar.focused {
            DRACULA_SELECTION
        } else {
            DRACULA_PURPLE
        }))
        .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));

    app.editor.textarea.set_block(block);
    app.editor
        .textarea
        .set_style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));
    app.editor
        .textarea
        .set_cursor_line_style(Style::default().bg(DRACULA_BG));
    frame.render_widget(&app.editor.textarea, area);
}
