use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::terminal::app::{App, InputMode};

const MOUSE_SCROLL_LINES: usize = 3;
const MENU_CLICK_WIDTH: u16 = 3;

fn build_project_content() -> String {
    // #320: both `fetch_dora_metrics` and `fetch_board` went async when
    // they moved from `gh` subprocess to octocrab. This callsite is the
    // synchronous mouse-click handler in the main event loop (via
    // `handle_terminal_event`). We bridge to async with
    // `block_in_place` + `Handle::current().block_on`, which lets the
    // tokio multi-thread runtime park this worker while the other
    // workers continue to drive the rest of the select! arms.
    //
    // Pre-#320 the same handler already blocked the event loop on a
    // synchronous `gh` subprocess, so this preserves (rather than
    // regresses) the current latency shape. The right fix is to defer
    // content-building off the click path entirely — tracked as
    // follow-up; out of #320's scope.
    let handle = tokio::runtime::Handle::current();
    let mut content = String::new();
    let metrics = tokio::task::block_in_place(|| {
        handle.block_on(crate::terminal::board::fetch_dora_metrics())
    });
    if let Some(metrics) = metrics {
        content.push_str(&metrics.format());
        content.push_str("\n\n---\n\n");
    }
    match crate::terminal::board::fetch_roadmap() {
        Ok(text) => content.push_str(&text),
        Err(_) => {
            let board = tokio::task::block_in_place(|| {
                handle.block_on(crate::terminal::board::fetch_board())
            });
            match board {
                Ok(text) => content.push_str(&text),
                Err(e) => content.push_str(&format!("Project info unavailable: {e}")),
            }
        }
    }
    content
}

fn handle_click(col: u16, row: u16, app: &mut App) {
    if let Some((sx, sy, sw, sh)) = app.layout.sidebar_area {
        if col >= sx && col < sx + sw && row >= sy && row < sy + sh {
            if row == sy {
                // Click on sidebar header — toggle closed
                app.toggle_sidebar();
            } else {
                let content_row = row.saturating_sub(sy + 1);
                let channels = app.sidebar_channels();
                let idx = content_row as usize;
                if idx < channels.len() {
                    // Chevron click-zone (#224): the trailing `›` sits
                    // in the last column inside the sidebar border. A
                    // click there on a direct peer row switches view;
                    // elsewhere on the row (the name) toggles focus.
                    let on_chevron = col + 2 >= sx + sw;
                    app.sidebar.selected = idx;
                    app.sidebar_select_at(on_chevron);
                }
            }
            return;
        }
    } else if let Some((mx, my)) = app.layout.menu_click_cell {
        // The left-edge click zone is now the visible `≡` glyph
        // at the top-left of the chat area. Narrowed from the old
        // `col < 3 on any row` catch-all, which triggered false
        // positives on any upper-left click. Keeping a 3-column-wide
        // target for Fitts's-law forgiveness.
        if row == my && col >= mx && col < mx + MENU_CLICK_WIDTH {
            app.toggle_sidebar();
            return;
        }
    }

    // Check if the click hit a "[+N lines]" truncation indicator (#110).
    // Reuses the truncation_indicators/expanded_messages pair; DepthLevel
    // sentinel value on the map is unused by truncation-expand rendering.
    if let Some(key) = app
        .msg
        .truncation_indicators
        .iter()
        .find(|(y, _)| *y == row)
        .map(|(_, k)| k.clone())
    {
        use crate::terminal::app_state::DepthLevel;
        use std::collections::hash_map::Entry;
        match app.msg.expanded_messages.entry(key) {
            Entry::Occupied(e) => {
                e.remove();
            }
            Entry::Vacant(e) => {
                e.insert(DepthLevel::RoleDefault);
            }
        }
        return;
    }

    // Check if the click hit a multi-depth cycle affordance (#285).
    // Depth state lives in the same expanded_messages map; absence
    // means "use initial_for_role." Click advances one step per the
    // Summary → RoleDefault → Deep → Summary cycle.
    if let Some(key) = app
        .msg
        .depth_indicators
        .iter()
        .find(|(y, _)| *y == row)
        .map(|(_, k)| k.clone())
    {
        use crate::terminal::app_state::DepthLevel;
        let role = app.identity.name.as_str();
        let current = app
            .msg
            .expanded_messages
            .get(&key)
            .copied()
            .unwrap_or_else(|| DepthLevel::initial_for_role(role));
        app.msg.expanded_messages.insert(key, current.next());
        return;
    }

    if let Some((cx, cy, cw, _)) = app.layout.chat_area
        && row == cy
        && col >= cx
        && col < cx + cw
    {
        if matches!(app.input_mode, InputMode::Project) {
            app.project.content = None;
            app.input_mode = InputMode::Chat;
        } else {
            app.project.content = Some(build_project_content());
            app.input_mode = InputMode::Project;
            app.request_board_refresh_if_stale();
        }
        return;
    }

    if let Some((_, sb_y, _, _)) = app.layout.status_bar_area
        && row == sb_y
    {
        if let Some((start, end)) = app.layout.ci_indicator_col_range
            && col >= start
            && col <= end
        {
            super::super::ci::open_in_browser(super::super::ci::github_actions_url());
            return;
        }
        if app
            .layout
            .help_hint_col
            .is_some_and(|hint_col| col >= hint_col)
        {
            app.input_mode = InputMode::Help;
        }
    }
}

pub(super) fn handle(
    mouse: &MouseEvent,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if matches!(app.input_mode, InputMode::Help) {
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            app.input_mode = InputMode::Chat;
        }
        return Ok(());
    }
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.scroll_up(MOUSE_SCROLL_LINES);
        }
        MouseEventKind::ScrollDown => {
            app.scroll_down(MOUSE_SCROLL_LINES);
        }
        MouseEventKind::Down(MouseButton::Left) => {
            handle_click(mouse.column, mouse.row, app);
        }
        _ => {}
    }
    Ok(())
}
