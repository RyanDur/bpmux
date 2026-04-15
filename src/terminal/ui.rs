use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Clear, Dataset, GraphType, Paragraph, Wrap},
};

use crate::session;
use crate::terminal::app::App;
use crate::terminal::board_view;
use crate::terminal::chat;
use crate::terminal::commands_help::format_command_list;
use crate::terminal::commands_tokens::{format_tokens_overlay, human};
use crate::terminal::icons;
use crate::terminal::sidebar;
use crate::terminal::status_bar;

const MIN_WIDTH_FOR_SIDEBAR: u16 = 60;
use crate::terminal::markdown::{render_md_message, wrap_line_with_indent};
use crate::terminal::theme::{
    DRACULA_BG, DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_PURPLE, DRACULA_SELECTION,
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
    // #191: show active topic in the input prompt so the user knows
    // which topic they're typing into.
    let topic_label = if app.topic.active == "general" {
        String::new()
    } else {
        format!(" ({})", app.topic.active)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(match &app.send_target {
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
    if app.display.show_help {
        // Source the command list from `SLASH_COMMANDS` (#164) so the
        // overlay and the `/help` slash command never drift. Two-column
        // form to fit alongside the keys section in the 24-row terminal
        // budget — full descriptions live in the `/help` chat output.
        let mut help_text = format_command_list(2);
        help_text.push_str(
            "\n\nKeys:\n  Enter    send / expand    Esc     clear input\n  \
Ctrl+U   scroll up        Ctrl+D  scroll down\n  \u{2325}1       channel sidebar   \
Ctrl+T  timestamps\n  Ctrl+C   quit (empty)     ?  this overlay\n\n\
click anywhere or press any key to close",
        );

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
fn render_tokens_overlay(frame: &mut Frame, app: &App, area: Rect) {
    if !app.tokens.show {
        return;
    }
    // #195: append usage table to the token text
    let usage_text = crate::terminal::commands_tokens::format_usage_table(&app.feature_usage);

    // Narrow/short terminal fallback — preserves the simpler text
    // overlay for constrained environments where a chart would be
    // unreadable.
    if area.width < 60 || area.height < 16 {
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
    let overlay_width = 52u16.min(area.width);
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
    let overlay_width = 72u16.min(area.width);
    let overlay_height = 20u16.min(area.height);
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
    // #195: footer area grows to fit usage table if present.
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
    // #195: append usage statistics if any features have been used.
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

fn render_project_panel(frame: &mut Frame, app: &App, area: Rect) {
    use crate::terminal::board::{StoryState, stories_by_state};

    // #196: show board data in the project panel when stories are loaded.
    if !app.board.stories.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" project ")
            .title_style(
                Style::default()
                    .fg(DRACULA_PURPLE)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(DRACULA_SELECTION))
            .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));

        let groups = stories_by_state(&app.board.stories);
        let mut lines: Vec<Line> = Vec::new();

        for (state, stories) in &groups {
            let color = match state {
                StoryState::Ready => crate::terminal::theme::DRACULA_GREEN,
                StoryState::InProgress => crate::terminal::theme::DRACULA_ORANGE,
                StoryState::Delivered => crate::terminal::theme::DRACULA_YELLOW,
                _ => DRACULA_COMMENT,
            };

            // Compressed: accepted + icebox show count only
            if matches!(state, StoryState::Accepted | StoryState::Icebox) {
                lines.push(Line::from(Span::styled(
                    format!("{} ({})", state.label(), stories.len()),
                    Style::default().fg(DRACULA_COMMENT),
                )));
                continue;
            }

            // Active states: header + stories
            lines.push(Line::from(Span::styled(
                format!("{} ({})", state.label(), stories.len()),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )));
            for story in stories.iter().take(5) {
                lines.push(Line::from(Span::styled(
                    story.short_name(area.width.saturating_sub(4) as usize),
                    Style::default().fg(DRACULA_FG),
                )));
            }
            if stories.len() > 5 {
                lines.push(Line::from(Span::styled(
                    format!("+{} more", stories.len() - 5),
                    Style::default().fg(DRACULA_COMMENT),
                )));
            }
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    // Fallback: existing project view content (roadmap/DORA)
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

    let content = app.project.content.as_deref().unwrap_or("Loading...");

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

    let sidebar_width = if app.sidebar.open && full.width >= MIN_WIDTH_FOR_SIDEBAR {
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
        app.layout.sidebar_area = Some((s.x, s.y, s.width, s.height));
        sidebar::render(frame, app, h_chunks[0]);
    } else {
        app.layout.sidebar_area = None;
    }
    app.layout.status_bar_area = Some((
        status_area.x,
        status_area.y,
        status_area.width,
        status_area.height,
    ));

    render_broadcast_banner(frame, app, chunks[0]);

    if app.project.show && chunks[1].width >= 60 {
        // Side-by-side: project view (60%) | chat (40%)
        let split = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);
        app.layout.chat_area = Some((split[0].x, split[0].y, split[0].width, split[0].height));
        render_project_panel(frame, app, split[0]);
        chat::render_chat(frame, app, split[1]);
        chat::render_unread_indicator(frame, app, split[1]);
    } else {
        let chat_rect = chunks[1];
        app.layout.chat_area = Some((chat_rect.x, chat_rect.y, chat_rect.width, chat_rect.height));
        chat::render_chat(frame, app, chunks[1]);
        chat::render_unread_indicator(frame, app, chunks[1]);
    }
    render_input(frame, app, chunks[2]);
    render_popup(frame, app, chunks[2]);
    status_bar::render(frame, app, status_area);

    // #183: when the sidebar is closed, overlay a `≡` glyph at the
    // top-left of the chat area. This is the "hamburger icon" that
    // replaces the previously invisible left-edge click zone — the
    // biggest discoverability win in the clickable-affordances spec.
    // Stores the click-cell coord on `app.layout.menu_click_cell` so the
    // mouse handler can detect clicks on just that cell rather than
    // the old "col < 3 on any row" catch-all.
    if sidebar_width == 0 {
        let menu_x = chunks[1].x;
        let menu_y = chunks[1].y;
        let menu_area = Rect::new(menu_x, menu_y, 1, 1);
        let menu = Paragraph::new(Span::styled(icons::MENU, Style::default().fg(DRACULA_CYAN)));
        frame.render_widget(menu, menu_area);
        app.layout.menu_click_cell = Some((menu_x, menu_y));
    } else {
        app.layout.menu_click_cell = None;
    }

    // #193: board view replaces chat area when active.
    board_view::render(frame, app, main_area);

    render_help_overlay(frame, app, main_area);
    render_tokens_overlay(frame, app, main_area);
    render_command_palette(frame, app, main_area);
}

fn render_command_palette(frame: &mut Frame, app: &App, area: Rect) {
    if !app.palette.show {
        return;
    }
    let matches = app.palette_matches();
    let palette_height = (matches.len() as u16 + 4).min(area.height); // +4 for border + input + gap
    let palette_width = 24u16.min(area.width);
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
