//! Board view renderer (#193) — glanceable project state.
//!
//! Two screens:
//! 1. Column layout showing stories grouped by state
//! 2. Detail view for a single story (metadata + ACs)

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::app::App;
use super::board::{BoardStory, StoryState, stories_by_state};
use super::theme::{
    DRACULA_BG, DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_GREEN, DRACULA_ORANGE,
    DRACULA_PURPLE, DRACULA_SELECTION, DRACULA_YELLOW,
};

fn state_color(state: StoryState) -> ratatui::style::Color {
    match state {
        StoryState::Ready => DRACULA_GREEN,
        StoryState::InProgress => DRACULA_ORANGE,
        StoryState::Delivered => DRACULA_YELLOW,
        StoryState::Accepted => DRACULA_COMMENT,
        StoryState::Deferred => DRACULA_PURPLE,
        StoryState::Icebox => DRACULA_COMMENT,
    }
}

/// Maximum stories shown per column before compressing.
const BOARD_COLUMNS: usize = 3;
const MAX_PER_COLUMN: usize = 6;

pub(super) fn render(frame: &mut Frame, app: &App, area: Rect) {
    if !matches!(app.input_mode, crate::terminal::app::InputMode::Board) {
        return;
    }
    if app.board.detail.is_some() {
        render_detail(frame, app, area);
    } else {
        render_columns(frame, app, area);
    }
}

/// Build a flat index of all visible stories for arrow-key navigation.
/// Returns (state, story_index_within_state) pairs matching what
/// render_columns shows.
pub(super) fn visible_stories(stories: &[BoardStory]) -> Vec<usize> {
    let groups = stories_by_state(stories);
    let mut indices = Vec::new();
    for (_, group) in &groups {
        let show_count = group.len().min(MAX_PER_COLUMN);
        for story in group.iter().take(show_count) {
            // Find the story's index in the original flat list
            if let Some(idx) = stories.iter().position(|s| s.number == story.number) {
                indices.push(idx);
            }
        }
    }
    indices
}

fn render_columns(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let groups = stories_by_state(&app.board.stories);
    let visible = visible_stories(&app.board.stories);

    // Layout: 3 columns top row, 3 columns bottom row
    // Split vertically first, then each half into 3 columns
    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2) as usize;
    let col_width = inner_width / BOARD_COLUMNS;
    let half_height = inner_height / 2;

    let mut lines: Vec<Line> = Vec::new();

    // Render in two rows of up to 3 columns each
    let top_states: Vec<StoryState> = vec![
        StoryState::Ready,
        StoryState::InProgress,
        StoryState::Delivered,
    ];
    let bottom_states: Vec<StoryState> = vec![
        StoryState::Accepted,
        StoryState::Deferred,
        StoryState::Icebox,
    ];

    // Render a row of columns
    let render_row = |states: &[StoryState],
                      groups: &[(StoryState, Vec<&BoardStory>)],
                      visible: &[usize],
                      selected: usize,
                      max_rows: usize|
     -> Vec<Line<'static>> {
        // Build column content (each column is a Vec of lines)
        let mut columns: Vec<Vec<(String, Style)>> = Vec::new();
        for &st in states {
            let mut col_lines: Vec<(String, Style)> = Vec::new();
            let group: Vec<&&BoardStory> = groups
                .iter()
                .filter(|(s, _)| *s == st)
                .flat_map(|(_, g)| g.iter())
                .collect();
            let count = group.len();

            // Header
            col_lines.push((
                format!("{} ({count})", st.label()),
                Style::default()
                    .fg(state_color(st))
                    .add_modifier(Modifier::BOLD),
            ));
            // Separator
            let header_len = st.label().len() + format!(" ({count})").len();
            col_lines.push((
                "─".repeat(header_len.min(col_width.saturating_sub(2))),
                Style::default().fg(DRACULA_COMMENT),
            ));

            if st == StoryState::Accepted && count > 3 {
                // Compress accepted: show count only
                col_lines.push((
                    format!("({count} stories)"),
                    Style::default().fg(DRACULA_COMMENT),
                ));
            } else if st == StoryState::Icebox && count > 3 {
                // Compress icebox: show count only
                col_lines.push((
                    format!("{count} stories"),
                    Style::default().fg(DRACULA_COMMENT),
                ));
            } else {
                for story in group.iter().take(MAX_PER_COLUMN) {
                    let is_selected = visible
                        .iter()
                        .position(|&idx| {
                            idx < app.board.stories.len()
                                && app.board.stories[idx].number == story.number
                        })
                        .is_some_and(|pos| pos == selected);
                    let mut name = story.short_name(col_width.saturating_sub(2));
                    if st == StoryState::Deferred && !story.reason.is_empty() {
                        name.push_str(&format!(" ({})", story.reason));
                    }
                    let style = if is_selected {
                        Style::default()
                            .fg(DRACULA_FG)
                            .bg(DRACULA_SELECTION)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(DRACULA_FG)
                    };
                    col_lines.push((name, style));
                }
                if count > MAX_PER_COLUMN {
                    col_lines.push((
                        format!("+{} more", count - MAX_PER_COLUMN),
                        Style::default().fg(DRACULA_COMMENT),
                    ));
                }
            }
            columns.push(col_lines);
        }

        // Merge columns side-by-side
        let max_lines = columns
            .iter()
            .map(|c| c.len())
            .max()
            .unwrap_or(0)
            .min(max_rows);

        let mut merged: Vec<Line<'static>> = Vec::new();
        for row in 0..max_lines {
            let mut spans: Vec<Span<'static>> = Vec::new();
            for (ci, col) in columns.iter().enumerate() {
                if ci > 0 {
                    spans.push(Span::raw("  ")); // column gap
                }
                if let Some((text, style)) = col.get(row) {
                    let padded = format!("{:width$}", text, width = col_width.saturating_sub(2));
                    spans.push(Span::styled(padded, *style));
                } else {
                    spans.push(Span::raw(" ".repeat(col_width.saturating_sub(2))));
                }
            }
            merged.push(Line::from(spans));
        }
        merged
    };

    lines.extend(render_row(
        &top_states,
        &groups,
        &visible,
        app.board.selected,
        half_height.saturating_sub(1),
    ));
    lines.push(Line::default()); // gap between rows
    lines.extend(render_row(
        &bottom_states,
        &groups,
        &visible,
        app.board.selected,
        half_height.saturating_sub(1),
    ));

    let board = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" board ")
            .title_style(
                Style::default()
                    .fg(DRACULA_CYAN)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(DRACULA_CYAN))
            .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
    );
    frame.render_widget(board, area);
}

fn render_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let visible = visible_stories(&app.board.stories);
    let story_idx = app.board.detail.unwrap_or(0);
    let global_idx = visible.get(story_idx).copied().unwrap_or(0);
    let story = match app.board.stories.get(global_idx) {
        Some(s) => s,
        None => return,
    };

    let mut lines: Vec<Line> = Vec::new();

    // Metadata row
    let state_style = Style::default()
        .fg(state_color(story.state))
        .add_modifier(Modifier::BOLD);
    lines.push(Line::from(vec![
        Span::styled("  State: ", Style::default().fg(DRACULA_COMMENT)),
        Span::styled(story.state.label(), state_style),
        Span::raw("              "),
        Span::styled("Filed: ", Style::default().fg(DRACULA_COMMENT)),
        Span::styled(story.created.clone(), Style::default().fg(DRACULA_FG)),
    ]));

    if !story.assignees.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Dev: ", Style::default().fg(DRACULA_COMMENT)),
            Span::styled(story.assignees.join(", "), Style::default().fg(DRACULA_FG)),
        ]));
    }

    lines.push(Line::default());

    // Description (first ~6 lines of body)
    let body_lines: Vec<&str> = story.body.lines().collect();
    let desc_end = body_lines
        .iter()
        .position(|l| {
            l.starts_with("## Acceptance") || l.starts_with("## AC") || l.starts_with("## Not")
        })
        .unwrap_or(body_lines.len().min(8));

    let inner_width = area.width.saturating_sub(4) as usize;
    for line in body_lines.iter().take(desc_end) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            lines.push(Line::default());
            continue;
        }
        let display: String = if trimmed.len() > inner_width {
            format!(
                "  {}…",
                trimmed.chars().take(inner_width - 3).collect::<String>()
            )
        } else {
            format!("  {trimmed}")
        };
        lines.push(Line::from(Span::styled(
            display,
            Style::default().fg(DRACULA_FG),
        )));
    }

    // Mockup reference
    let has_mockup = body_lines
        .iter()
        .any(|l| l.contains("docs/design") || l.contains("mockup"));
    if has_mockup {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "  Mockups: see issue body",
            Style::default().fg(DRACULA_COMMENT),
        )));
    }

    // AC checklist
    let ac_start = body_lines
        .iter()
        .position(|l| l.starts_with("## Acceptance") || l.starts_with("## AC"));
    if let Some(start) = ac_start {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "  ───────────────────────────────────────────────────────��──",
            Style::default().fg(DRACULA_COMMENT),
        )));
        lines.push(Line::from(Span::styled(
            "  ACs:",
            Style::default()
                .fg(DRACULA_CYAN)
                .add_modifier(Modifier::BOLD),
        )));

        for line in body_lines.iter().skip(start + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("## ") {
                break; // next section
            }
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                let ac_text = &trimmed[2..];
                // Check if AC is in delivered/accepted labels → ✓
                let is_done =
                    story.state == StoryState::Accepted || story.state == StoryState::Delivered;
                let (marker, style) = if is_done {
                    ("  ✓ ", Style::default().fg(DRACULA_GREEN))
                } else {
                    ("    ", Style::default().fg(DRACULA_COMMENT))
                };
                let display: String = if ac_text.len() > inner_width.saturating_sub(4) {
                    format!(
                        "{}…",
                        ac_text
                            .chars()
                            .take(inner_width.saturating_sub(5))
                            .collect::<String>()
                    )
                } else {
                    ac_text.to_string()
                };
                lines.push(Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled(display, style),
                ]));
            }
        }
    }

    let title = format!(" #{} {} ", story.number, story.title);
    let detail = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(
                Style::default()
                    .fg(DRACULA_CYAN)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(DRACULA_CYAN))
            .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
    );
    frame.render_widget(detail, area);
}
