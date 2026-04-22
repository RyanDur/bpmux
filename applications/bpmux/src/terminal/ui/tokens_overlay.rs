use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Clear, Dataset, GraphType, Paragraph},
};

use super::help_overlay::HELP_OVERLAY_WIDTH;
use crate::terminal::app::App;
use crate::terminal::commands::tokens::{format_tokens_overlay, human};
use crate::terminal::theme::{DRACULA_BG, DRACULA_COMMENT, DRACULA_FG, DRACULA_PURPLE};

const TOKENS_OVERLAY_WIDTH: u16 = 72;
const TOKENS_OVERLAY_HEIGHT: u16 = 20;
const MIN_CHART_WIDTH: u16 = 60;
const MIN_CHART_HEIGHT: u16 = 16;

/// Render the `/tokens` overlay. Uses a ratatui `Chart` widget for
/// the line-graph-over-time Ryan asked for (#179) when the terminal
/// has enough room, and falls back to the plain text formatter for
/// narrow / short terminals so the overlay is always useful.
///
/// Layout (chart mode):
/// ```text
/// ┌ tokens ──────────────────────────────────────────┐
/// │ Total: 12.3K across 14 rows                      │
/// │ By role:                                         │
/// │   dev       8.1K                                 │
/// │   pm        2.4K                                 │
/// │   designer  1.8K                                 │
/// │ ┌─ per turn ────────────────────────────────────┐│
/// │ │                         •                     ││
/// │ │              •       •     •                  ││
/// │ │      •                        •    •          ││
/// │ │                                        •      ││
/// │ │ 0                                         N-1 ││
/// │ └───────────────────────────────────────────────┘│
/// │ Press any key to close                           │
/// └──────────────────────────────────────────────────┘
/// ```
pub(super) fn render_tokens_overlay(frame: &mut Frame, app: &App, area: Rect) {
    if !matches!(app.input_mode, crate::terminal::app::InputMode::Tokens) {
        return;
    }
    // Append usage table to the token text
    let usage_text = crate::terminal::commands::tokens::format_usage_table(&app.feature_usage);

    // Narrow/short terminal fallback — preserves the simpler text
    // overlay for constrained environments where a chart would be
    // unreadable.
    if area.width < MIN_CHART_WIDTH || area.height < MIN_CHART_HEIGHT {
        render_tokens_text_fallback(frame, &app.tokens.rows, &usage_text, area);
        return;
    }
    render_tokens_chart_view(frame, &app.tokens.rows, &usage_text, area);
}

fn render_tokens_text_fallback(
    frame: &mut Frame,
    rows: &[crate::session::tokens::TokenRow],
    usage_text: &str,
    area: Rect,
) {
    let mut text = format_tokens_overlay(rows);
    text.push_str(usage_text);
    text.push_str("Press any key to close");
    let lines: Vec<Line> = text
        .lines()
        .map(|l| Line::from(Span::raw(l.to_string())))
        .collect();
    let overlay_height = (lines.len() as u16 + 2).min(area.height);
    let overlay_width = HELP_OVERLAY_WIDTH.min(area.width);
    let overlay_x = area.x + (area.width.saturating_sub(overlay_width)) / 2;
    let overlay_y = area.y + (area.height.saturating_sub(overlay_height)) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);
    let popup = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" tokens ")
            .title_style(
                Style::default()
                    .fg(DRACULA_PURPLE)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(DRACULA_PURPLE))
            .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
    );
    frame.render_widget(Clear, overlay_area);
    frame.render_widget(popup, overlay_area);
}

fn render_tokens_chart_view(
    frame: &mut Frame,
    rows: &[crate::session::tokens::TokenRow],
    usage_text: &str,
    area: Rect,
) {
    use crate::session::tokens::totals_by_role;

    // Size the overlay generously but keep it centered and capped so
    // it doesn't fill the whole screen — Ryan can still see chat
    // context hints around the edges even when the overlay is up.
    let overlay_width = TOKENS_OVERLAY_WIDTH.min(area.width);
    let overlay_height = TOKENS_OVERLAY_HEIGHT.min(area.height);
    let overlay_x = area.x + area.width.saturating_sub(overlay_width) / 2;
    let overlay_y = area.y + area.height.saturating_sub(overlay_height) / 2;
    let overlay_rect = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    frame.render_widget(Clear, overlay_rect);

    let title = if usage_text.is_empty() {
        " tokens "
    } else {
        " tokens + usage "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(DRACULA_PURPLE)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(DRACULA_PURPLE))
        .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));
    let inner = block.inner(overlay_rect);
    frame.render_widget(block, overlay_rect);

    // Vertical split: header (1) / breakdown (variable, up to 6) /
    // chart (fills remaining) / footer (1).
    let totals = totals_by_role(rows);
    let breakdown_height = if totals.is_empty() {
        2 // "(no rows yet)"
    } else {
        (totals.len() as u16 + 1).min(6) // "By role:" line + entries
    };
    // Footer area grows to fit usage table if present.
    let usage_line_count = usage_text.lines().count() as u16;
    let footer_height = (1 + usage_line_count).max(1);
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(breakdown_height),
        Constraint::Min(4),
        Constraint::Length(footer_height),
    ])
    .split(inner);

    // ── Header: grand total ──
    let total: u64 = rows.iter().map(|r| r.total_tokens()).sum();
    let header = format!(
        "Total: {} tokens across {} row{}",
        human(total),
        rows.len(),
        if rows.len() == 1 { "" } else { "s" }
    );
    frame.render_widget(
        Paragraph::new(header).style(Style::default().fg(DRACULA_FG)),
        chunks[0],
    );

    // ── Breakdown: per-role totals ──
    if totals.is_empty() {
        let lines = vec![
            Line::from("No token rows yet."),
            Line::from("Once an agent completes a turn with the adapter hook, rows appear here."),
        ];
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().fg(DRACULA_COMMENT)),
            chunks[1],
        );
    } else {
        let mut lines = vec![Line::from(Span::styled(
            "By role:",
            Style::default().fg(DRACULA_FG),
        ))];
        let widest = totals.keys().map(|k| k.len()).max().unwrap_or(0);
        for (role, t) in &totals {
            lines.push(Line::from(format!(
                "  {role:<widest$}  {}",
                human(*t),
                widest = widest
            )));
        }
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().fg(DRACULA_FG)),
            chunks[1],
        );
    }

    // ── Chart: per-turn totals over turn index ──
    // Ratatui Chart borrows the data vec for the duration of the
    // render call, so we build it inline and hand it to Dataset.
    let data: Vec<(f64, f64)> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| (i as f64, r.total_tokens() as f64))
        .collect();

    if rows.len() >= 2 {
        let max_x = (rows.len() - 1) as f64;
        let max_y = data.iter().map(|(_, y)| *y).fold(1.0_f64, f64::max);
        let datasets = vec![
            Dataset::default()
                .name("per turn")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(DRACULA_PURPLE))
                .data(&data),
        ];
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .title(" per turn ")
                    .title_style(Style::default().fg(DRACULA_COMMENT))
                    .border_style(Style::default().fg(DRACULA_COMMENT)),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(DRACULA_COMMENT))
                    .bounds([0.0, max_x])
                    .labels(vec![Line::from("0"), Line::from(max_x.to_string())]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(DRACULA_COMMENT))
                    .bounds([0.0, max_y])
                    .labels(vec![Line::from("0"), Line::from(human(max_y as u64))]),
            );
        frame.render_widget(chart, chunks[2]);
    } else {
        let msg = if rows.is_empty() {
            "(adapter hook must populate rows first — see docs/token-log-schema.md)"
        } else {
            "(need 2+ rows to chart a trend)"
        };
        frame.render_widget(
            Paragraph::new(msg).style(Style::default().fg(DRACULA_COMMENT)),
            chunks[2],
        );
    }

    // ── Footer: usage table + dismiss hint ──
    // Append usage statistics if any features have been used.
    let mut footer_text = String::new();
    footer_text.push_str(usage_text);
    footer_text.push_str("Press any key to close");
    let footer_lines: Vec<Line> = footer_text
        .lines()
        .map(|l| Line::from(Span::raw(l.to_string())))
        .collect();
    frame.render_widget(
        Paragraph::new(footer_lines).style(Style::default().fg(DRACULA_COMMENT)),
        chunks[3],
    );
}
