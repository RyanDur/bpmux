use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::session;
use crate::terminal::app::App;
use crate::terminal::chat;
use crate::terminal::sidebar;
use crate::terminal::status_bar;

const MIN_WIDTH_FOR_SIDEBAR: u16 = 60;
use crate::terminal::markdown::{render_md_message, wrap_line_with_indent};
use crate::terminal::theme::{
    DRACULA_BG, DRACULA_COMMENT, DRACULA_FG, DRACULA_PURPLE, DRACULA_SELECTION,
};

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

fn render_input(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
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
        .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));

    app.textarea.set_block(block);
    app.textarea
        .set_style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));
    app.textarea
        .set_cursor_line_style(Style::default().bg(DRACULA_BG));
    frame.render_widget(&app.textarea, area);
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

fn render_project_panel(frame: &mut Frame, app: &App, area: Rect) {
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

    let content = app.project_view_content.as_deref().unwrap_or("Loading...");

    let inner_width = area.width.saturating_sub(2) as usize;
    let prefix_spans = Vec::new();
    let indent = Span::raw("");
    let md_lines = render_md_message(content, prefix_spans, indent.clone(), None, inner_width);
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
        sidebar::render(frame, app, h_chunks[0]);
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
        chat::render_chat(frame, app, split[1]);
        chat::render_unread_indicator(frame, app, split[1]);
    } else {
        let chat_rect = chunks[1];
        app.chat_area = Some((chat_rect.x, chat_rect.y, chat_rect.width, chat_rect.height));
        chat::render_chat(frame, app, chunks[1]);
        chat::render_unread_indicator(frame, app, chunks[1]);
    }
    render_input(frame, app, chunks[2]);
    render_popup(frame, app, chunks[2]);
    status_bar::render(frame, app, status_area);
    render_help_overlay(frame, app, main_area);
}
