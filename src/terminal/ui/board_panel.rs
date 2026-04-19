//! Right-panel board renderer (#318). Compact vertical list of
//! stories grouped by state, sized for the 28-30 col side panel
//! that coexists with chat. Distinct from the full-screen
//! `board_view::render` which uses a 3-column layout and only
//! applies when `InputMode::Board` is active.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::terminal::app::App;
use crate::terminal::board::{StoryState, stories_by_state};
use crate::terminal::theme::{
    DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_GREEN, DRACULA_ORANGE, DRACULA_PURPLE,
    DRACULA_YELLOW,
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" board ")
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

    let title_budget = (inner.width as usize).saturating_sub(9); // "#NNN rdy " = ~9 chars
    let mut lines: Vec<Line> = Vec::new();
    let groups = stories_by_state(&app.board.stories);
    for (state, stories) in &groups {
        if stories.is_empty() {
            continue;
        }
        let (badge_text, badge_color) = state_badge(*state);
        for story in stories {
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
                    Style::default().fg(DRACULA_COMMENT),
                ),
                Span::styled(format!("{badge_text} "), Style::default().fg(badge_color)),
                Span::styled(title_display, Style::default().fg(DRACULA_FG)),
            ]));
        }
    }
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
