//! Right-panel board renderer (#318, #319). Two modes:
//!   - row-list view: compact vertical list of stories grouped by
//!     state, sized for the 28-30 col side panel
//!   - drill-in view (#319): full detail for the selected story —
//!     title, type, state, labels, assignees, body (scrollable).
//!     Comments are a follow-up Feature; not yet rendered.
//!
//! Distinct from the full-screen `board_view::render` which uses a
//! wide 3-column layout.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::terminal::app::App;
use crate::terminal::board::{BoardStory, StoryState, stories_by_state};
use crate::terminal::theme::{
    DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_GREEN, DRACULA_ORANGE, DRACULA_PURPLE,
    DRACULA_SELECTION, DRACULA_YELLOW,
};

fn state_badge(state: StoryState) -> (&'static str, ratatui::style::Color) {
    match state {
        StoryState::Ready => ("rdy", DRACULA_GREEN),
        StoryState::InProgress => ("wip", DRACULA_ORANGE),
        StoryState::Delivered => ("dlv", DRACULA_YELLOW),
        StoryState::Accepted => ("acc", DRACULA_COMMENT),
        StoryState::Deferred => ("def", DRACULA_PURPLE),
        StoryState::Icebox => ("ice", DRACULA_COMMENT),
    }
}

pub(super) fn render(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.board.panel_focused {
        " board ▸ "
    } else {
        " board "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(DRACULA_CYAN)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.board.stories.is_empty() {
        let loading = Paragraph::new("loading...").style(Style::default().fg(DRACULA_COMMENT));
        frame.render_widget(loading, inner);
        return;
    }

    if let Some(idx) = app.board.panel_detail
        && let Some(story) = app.board.stories.get(idx)
    {
        render_detail(frame, inner, story, app.board.panel_detail_scroll);
        return;
    }

    render_list(frame, inner, app);
}

fn render_list(frame: &mut Frame, inner: Rect, app: &App) {
    let title_budget = (inner.width as usize).saturating_sub(10);
    let mut lines: Vec<Line> = Vec::new();
    let mut row_idx: usize = 0;
    let groups = stories_by_state(&app.board.stories);
    for (state, stories) in &groups {
        if stories.is_empty() {
            continue;
        }
        let (badge_text, badge_color) = state_badge(*state);
        for story in stories {
            let absolute_idx = app
                .board
                .stories
                .iter()
                .position(|s| s.number == story.number)
                .unwrap_or(row_idx);
            let selected = absolute_idx == app.board.panel_selected && app.board.panel_focused;
            let bg = if selected {
                DRACULA_SELECTION
            } else {
                ratatui::style::Color::Reset
            };
            let title_display = if story.title.chars().count() > title_budget {
                let trunc: String = story
                    .title
                    .chars()
                    .take(title_budget.saturating_sub(1))
                    .collect();
                format!("{trunc}…")
            } else {
                story.title.clone()
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("#{:<4}", story.number),
                    Style::default().fg(DRACULA_COMMENT).bg(bg),
                ),
                Span::styled(
                    format!("{badge_text} "),
                    Style::default().fg(badge_color).bg(bg),
                ),
                Span::styled(title_display, Style::default().fg(DRACULA_FG).bg(bg)),
            ]));
            row_idx += 1;
        }
    }
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_detail(frame: &mut Frame, inner: Rect, story: &BoardStory, scroll: u16) {
    let (badge_text, badge_color) = state_badge(story.state);
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            format!("#{} ", story.number),
            Style::default().fg(DRACULA_COMMENT),
        ),
        Span::styled(
            badge_text,
            Style::default()
                .fg(badge_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        story.title.clone(),
        Style::default().fg(DRACULA_FG).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    if !story.labels.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("labels: ", Style::default().fg(DRACULA_COMMENT)),
            Span::styled(story.labels.join(", "), Style::default().fg(DRACULA_CYAN)),
        ]));
    }
    if !story.assignees.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("assignees: ", Style::default().fg(DRACULA_COMMENT)),
            Span::styled(
                story.assignees.join(", "),
                Style::default().fg(DRACULA_CYAN),
            ),
        ]));
    }
    if !story.created.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("created: ", Style::default().fg(DRACULA_COMMENT)),
            Span::styled(story.created.clone(), Style::default().fg(DRACULA_COMMENT)),
        ]));
    }
    lines.push(Line::from(""));
    // Body is pre-fetched by the board poller into BoardStory.body;
    // split on newlines for paragraph rendering (with wrap).
    for body_line in story.body.lines() {
        lines.push(Line::from(body_line.to_string()));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Esc: back · ↑↓/Ctrl+U/D: scroll",
        Style::default().fg(DRACULA_COMMENT),
    )));

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, inner);
}
