mod board_panel;
mod broadcast;
mod help_overlay;
mod input_box;
mod metrics;
mod palette;
mod popup;
mod project_panel;
mod tokens_overlay;

#[cfg(test)]
pub(super) use metrics::{fmt_opt_hours, fmt_opt_rate};

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Span,
    widgets::Paragraph,
};

use crate::terminal::app::App;
use crate::terminal::board_view;
use crate::terminal::chat;
use crate::terminal::icons;
use crate::terminal::sidebar;
use crate::terminal::status_bar;
use crate::terminal::theme::DRACULA_CYAN;

const MIN_WIDTH_FOR_SIDEBAR: u16 = 60;
const SIDEBAR_WIDTH: u16 = 24;
const MAX_INPUT_HEIGHT: u16 = 8;
const MIN_SPLIT_VIEW_WIDTH: u16 = 60;
/// Side-by-side board panel (#318) — width reserved on the right
/// when `app.board.panel_open` is true and the terminal is wide
/// enough to fit chat + panel. At narrower terminals the panel
/// falls back to hidden so chat doesn't get compressed below its
/// legibility floor.
const BOARD_PANEL_WIDTH: u16 = 30;
/// Minimum terminal width required for chat + board panel to
/// coexist. At <80 cols chat compresses below ~45 cols which breaks
/// readability; honest degrade is "chat-only until you resize."
const MIN_WIDTH_FOR_BOARD_PANEL: u16 = 80;

pub fn render(frame: &mut Frame, app: &mut App) {
    let full = frame.area();

    // Top-level split: content area + full-width status bar
    let v_split = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(full);
    let content_area = v_split[0];
    let status_area = v_split[1];

    let sidebar_width = if app.sidebar.open && full.width >= MIN_WIDTH_FOR_SIDEBAR {
        SIDEBAR_WIDTH
    } else {
        0
    };
    let board_panel_width = if app.board.panel_open && full.width >= MIN_WIDTH_FOR_BOARD_PANEL {
        BOARD_PANEL_WIDTH
    } else {
        0
    };

    let h_chunks = Layout::horizontal([
        Constraint::Length(sidebar_width),
        Constraint::Min(1),
        Constraint::Length(board_panel_width),
    ])
    .split(content_area);

    let main_area = h_chunks[1];
    let board_panel_area = h_chunks[2];

    let input_lines = if app.editor.input.is_empty() {
        1
    } else {
        let width = main_area.width.saturating_sub(2) as usize;
        app.editor.input.len().max(1).div_ceil(width.max(1))
    };
    let input_height = (input_lines + 2).min(MAX_INPUT_HEIGHT as usize) as u16;

    let banner_height = broadcast::broadcast_banner_height(app, main_area.width);

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

    broadcast::render_broadcast_banner(frame, app, chunks[0]);

    if matches!(app.input_mode, crate::terminal::app::InputMode::Project)
        && chunks[1].width >= MIN_SPLIT_VIEW_WIDTH
    {
        // Side-by-side: project view (60%) | chat (40%)
        let split = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);
        app.layout.chat_area = Some((split[0].x, split[0].y, split[0].width, split[0].height));
        project_panel::render_project_panel(frame, app, split[0]);
        chat::render_chat(frame, app, split[1]);
        chat::render_unread_indicator(frame, app, split[1]);
    } else {
        let chat_rect = chunks[1];
        app.layout.chat_area = Some((chat_rect.x, chat_rect.y, chat_rect.width, chat_rect.height));
        chat::render_chat(frame, app, chunks[1]);
        chat::render_unread_indicator(frame, app, chunks[1]);
    }
    input_box::render_input(frame, app, chunks[2]);
    popup::render_popup(frame, app, chunks[2]);
    status_bar::render(frame, app, status_area);

    // When the sidebar is closed, overlay a `≡` glyph at the
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

    // Side-by-side board panel (#318) — coexists with chat when
    // `panel_open` and the terminal fits both. Rendered outside the
    // `main_area` used by chat / project overlays.
    if board_panel_width > 0 {
        board_panel::render(frame, app, board_panel_area);
    }

    // Board view replaces chat area when active.
    board_view::render(frame, app, main_area);

    help_overlay::render_help_overlay(frame, app, main_area);
    tokens_overlay::render_tokens_overlay(frame, app, main_area);
    palette::render_command_palette(frame, app, main_area);
}
