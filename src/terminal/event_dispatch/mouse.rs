use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::terminal::app::{App, InputMode};

const MOUSE_SCROLL_LINES: usize = 3;
const MENU_CLICK_WIDTH: u16 = 3;

fn build_project_content() -> String {
    let mut content = String::new();
    if let Some(metrics) = crate::terminal::board::fetch_dora_metrics() {
        content.push_str(&metrics.format());
        content.push_str("\n\n---\n\n");
    }
    match crate::terminal::board::fetch_roadmap() {
        Ok(text) => content.push_str(&text),
        Err(_) => match crate::terminal::board::fetch_board() {
            Ok(text) => content.push_str(&text),
            Err(e) => content.push_str(&format!("Project info unavailable: {e}")),
        },
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
                    app.sidebar.selected = idx;
                    app.sidebar_select();
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

    // Check if the click hit a "[+N lines]" truncation indicator
    if let Some(key) = app
        .msg
        .truncation_indicators
        .iter()
        .find(|(y, _)| *y == row)
        .map(|(_, k)| k.clone())
    {
        if app.msg.expanded_messages.contains(&key) {
            app.msg.expanded_messages.remove(&key);
        } else {
            app.msg.expanded_messages.insert(key);
        }
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
        }
        return;
    }

    if let Some((_, sb_y, _, _)) = app.layout.status_bar_area
        && row == sb_y
        && app
            .layout
            .help_hint_col
            .is_some_and(|hint_col| col >= hint_col)
    {
        app.input_mode = InputMode::Help;
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
