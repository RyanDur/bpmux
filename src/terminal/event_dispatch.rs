//! Keyboard and mouse event dispatch extracted from event.rs.
//!
//! These functions handle the conversion from crossterm events to App state
//! mutations (cursor moves, scroll, sidebar toggles, slash-command submission)
//! and mouse-to-region routing. They live in their own file so event.rs can
//! stay focused on the tokio::select! event loop.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};

use crate::session;
use crate::terminal::app::App;

const PAGE_SCROLL_LINES: usize = 10;

fn bell() {
    use std::io::Write;
    if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        let _ = tty.write_all(b"\x07");
        let _ = tty.flush();
    }
}

fn scroll_up_with_bell(app: &mut App, amount: usize) {
    let before = app.scroll_offset;
    app.scroll_up(amount);
    if app.scroll_offset == before && before > 0 && !app.at_boundary {
        bell();
        app.at_boundary = true;
    }
}

fn scroll_down_with_bell(app: &mut App, amount: usize) {
    let before = app.scroll_offset;
    app.scroll_down(amount);
    if app.scroll_offset == before && !app.at_boundary {
        bell();
        app.at_boundary = true;
    }
}

fn build_project_content() -> String {
    let mut content = String::new();
    if let Some(metrics) = super::board::fetch_dora_metrics() {
        content.push_str(&metrics.format());
        content.push_str("\n\n---\n\n");
    }
    match super::board::fetch_roadmap() {
        Ok(text) => content.push_str(&text),
        Err(_) => match super::board::fetch_board() {
            Ok(text) => content.push_str(&text),
            Err(e) => content.push_str(&format!("Project info unavailable: {e}")),
        },
    }
    content
}

fn handle_mouse_click(col: u16, row: u16, app: &mut App) {
    if let Some((sx, sy, sw, sh)) = app.sidebar_area {
        if col >= sx && col < sx + sw && row >= sy && row < sy + sh {
            if row == sy {
                // Click on sidebar header — toggle closed
                app.toggle_sidebar();
            } else {
                let content_row = row.saturating_sub(sy + 1);
                let channels = app.sidebar_channels();
                let idx = content_row as usize;
                if idx < channels.len() {
                    app.sidebar_selected = idx;
                    app.sidebar_select();
                }
            }
            return;
        }
    } else if col < 3 {
        app.toggle_sidebar();
        return;
    }

    // Check if the click hit a "[+N lines]" truncation indicator
    if let Some(key) = app
        .truncation_indicators
        .iter()
        .find(|(y, _)| *y == row)
        .map(|(_, k)| k.clone())
    {
        if app.expanded_messages.contains(&key) {
            app.expanded_messages.remove(&key);
        } else {
            app.expanded_messages.insert(key);
        }
        return;
    }

    if let Some((cx, cy, cw, _)) = app.chat_area
        && row == cy
        && col >= cx
        && col < cx + cw
    {
        app.show_project_view = !app.show_project_view;
        if app.show_project_view {
            app.project_view_content = Some(build_project_content());
        } else {
            app.project_view_content = None;
        }
        return;
    }

    if let Some((_, sb_y, _, _)) = app.status_bar_area
        && row == sb_y
        && app.help_hint_col.is_some_and(|hint_col| col >= hint_col)
    {
        app.show_help = true;
    }
}

pub(super) fn handle_terminal_event(
    ev: Event,
    app: &mut App,
    session: &session::Session,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if matches!(&ev, Event::FocusGained) {
        app.tui_focused = true;
        return Ok(());
    }
    if matches!(&ev, Event::FocusLost) {
        app.tui_focused = false;
        return Ok(());
    }
    if let Event::Mouse(mouse) = &ev {
        if app.show_help {
            if mouse.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                app.show_help = false;
            }
            return Ok(());
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.scroll_up(3);
            }
            MouseEventKind::ScrollDown => {
                app.scroll_down(3);
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                handle_mouse_click(mouse.column, mouse.row, app);
            }
            _ => {}
        }
        return Ok(());
    }
    if let Event::Paste(text) = ev {
        for ch in text.chars() {
            app.insert_char(ch);
        }
        return Ok(());
    }
    let Event::Key(key) = ev else {
        return Ok(());
    };
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }
    if app.show_help {
        app.show_help = false;
        return Ok(());
    }
    if app.show_tokens_view {
        // Any key dismisses, same UX as the `?` help overlay.
        app.show_tokens_view = false;
        app.tokens_view_rows.clear();
        return Ok(());
    }
    if app.show_project_view && key.code == KeyCode::Esc {
        app.show_project_view = false;
        app.project_view_content = None;
        return Ok(());
    }
    if (key.code == KeyCode::Char('b') || key.code == KeyCode::Char('1'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        app.toggle_sidebar();
        return Ok(());
    }
    if key.code == KeyCode::Char('1') && key.modifiers.contains(KeyModifiers::ALT) {
        app.toggle_sidebar();
        return Ok(());
    }
    if app.sidebar_focused {
        match key.code {
            KeyCode::Up => app.sidebar_up(),
            KeyCode::Down => app.sidebar_down(),
            KeyCode::Left => app.sidebar_collapse(),
            KeyCode::Right => app.sidebar_expand(),
            KeyCode::Enter => app.sidebar_select(),
            KeyCode::Esc => app.sidebar_close(),
            KeyCode::Backspace => app.sidebar_filter_backspace(),
            KeyCode::Char(c) => app.sidebar_filter_char(c),
            _ => {}
        }
        return Ok(());
    }
    let popup_open = app.popup_visible();
    match key.code {
        KeyCode::Up if key.modifiers.is_empty() && popup_open => {
            app.popup_up();
        }
        KeyCode::Down if key.modifiers.is_empty() && popup_open => {
            app.popup_down();
        }
        KeyCode::Tab if popup_open && app.popup_selected.is_some() => {
            app.popup_accept();
        }
        KeyCode::Enter if popup_open && app.popup_selected.is_some() => {
            app.popup_accept();
        }
        KeyCode::Esc if popup_open => {
            app.popup_dismiss();
            app.clear_input();
        }
        KeyCode::Enter => {
            if app.input.is_empty() {
                app.toggle_last_truncated();
            } else if let Some(msg) = app.submit_message() {
                session.broadcast(&msg)?;
            }
        }
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.show_timestamps = !app.show_timestamps;
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            scroll_up_with_bell(app, PAGE_SCROLL_LINES);
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            scroll_down_with_bell(app, PAGE_SCROLL_LINES);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.input.is_empty() {
                app.should_quit = true;
                return Ok(());
            }
            app.clear_input();
        }
        KeyCode::Up if key.modifiers.is_empty() => {
            app.history_up();
        }
        KeyCode::Down if key.modifiers.is_empty() => {
            app.history_down();
        }
        KeyCode::Left if key.modifiers.is_empty() => {
            app.move_cursor_left();
        }
        KeyCode::Right if key.modifiers.is_empty() => {
            app.move_cursor_right();
        }
        KeyCode::Home => {
            app.move_cursor_home();
        }
        KeyCode::End => {
            app.move_cursor_end();
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.move_cursor_home();
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.move_cursor_end();
        }
        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.kill_to_end();
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.delete_word_backward();
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
            app.move_word_left();
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
            app.move_word_right();
        }
        KeyCode::Tab => {
            if popup_open {
                app.popup_down();
            } else {
                app.tab_complete();
            }
        }
        KeyCode::Char('?') if app.input.is_empty() => {
            app.show_help = true;
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        KeyCode::Backspace => {
            app.delete_char();
        }
        KeyCode::PageUp => {
            scroll_up_with_bell(app, 5);
        }
        KeyCode::PageDown => {
            scroll_down_with_bell(app, 5);
        }
        KeyCode::Esc => {
            app.clear_input();
        }
        _ => {}
    }
    Ok(())
}
