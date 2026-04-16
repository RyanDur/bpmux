use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use crate::session;
use crate::terminal::app::App;
use crate::terminal::theme::{DRACULA_COMMENT, DRACULA_FG, DRACULA_SELECTION};

const MAX_BANNER_LINES: u16 = 4;

fn latest_broadcast(app: &App) -> Option<&session::Message> {
    app.msg
        .messages
        .iter()
        .rev()
        .find(|m| m.is_chat() && m.to.is_none())
}

pub(super) fn broadcast_banner_height(app: &App, width: u16) -> u16 {
    if app
        .channels
        .current_view
        .as_ref()
        .is_none_or(|v| v == "broadcast")
    {
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

pub(super) fn render_broadcast_banner(frame: &mut Frame, app: &App, area: Rect) {
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
