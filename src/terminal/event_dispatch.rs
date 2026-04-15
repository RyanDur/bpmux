//! Keyboard and mouse event dispatch extracted from event.rs.
//!
//! These functions handle the conversion from crossterm events to App state
//! mutations (cursor moves, scroll, sidebar toggles, slash-command submission)
//! and mouse-to-region routing. They live in their own file so event.rs can
//! stay focused on the tokio::select! event loop.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};

use crate::session;
use crate::terminal::app::{App, InputMode};

const PAGE_SCROLL_LINES: usize = 10;
const MOUSE_SCROLL_LINES: usize = 3;
const MENU_CLICK_WIDTH: u16 = 3;

fn bell() {
    use std::io::Write;
    if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        let _ = tty.write_all(b"\x07");
        let _ = tty.flush();
    }
}

fn scroll_up_with_bell(app: &mut App, amount: usize) {
    let before = app.scroll.offset;
    app.scroll_up(amount);
    if app.scroll.offset == before && before > 0 && !app.scroll.at_boundary {
        bell();
        app.scroll.at_boundary = true;
    }
}

fn scroll_down_with_bell(app: &mut App, amount: usize) {
    let before = app.scroll.offset;
    app.scroll_down(amount);
    if app.scroll.offset == before && !app.scroll.at_boundary {
        bell();
        app.scroll.at_boundary = true;
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
        // #183: the left-edge click zone is now the visible `≡` glyph
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
        app.project.show = !app.project.show;
        if app.project.show {
            app.project.content = Some(build_project_content());
        } else {
            app.project.content = None;
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
        app.display.show_help = true;
        app.input_mode = InputMode::Help;
    }
}

pub(super) fn handle_terminal_event(
    ev: Event,
    app: &mut App,
    session: &session::Session,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if matches!(&ev, Event::FocusGained) {
        app.display.tui_focused = true;
        return Ok(());
    }
    if matches!(&ev, Event::FocusLost) {
        app.display.tui_focused = false;
        return Ok(());
    }
    if let Event::Mouse(mouse) = &ev {
        if app.display.show_help {
            if mouse.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                app.display.show_help = false;
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
    // Modal dispatch via InputMode enum — explicit state machine
    // replacing the old implicit priority chain of boolean flags.
    match app.input_mode {
        InputMode::Help => {
            app.display.show_help = false;
            app.input_mode = InputMode::Chat;
            return Ok(());
        }
        InputMode::Tokens => {
            app.tokens.show = false;
            app.tokens.rows.clear();
            app.input_mode = InputMode::Chat;
            return Ok(());
        }
        InputMode::Palette => {
            match key.code {
                KeyCode::Esc => {
                    app.palette.show = false;
                    app.palette.input.clear();
                    app.input_mode = InputMode::Chat;
                }
                KeyCode::Enter => {
                    let matches = app.palette_matches();
                    if let Some((cmd, _)) = matches.get(app.palette.selected) {
                        let command = cmd.to_string();
                        app.palette.show = false;
                        app.palette.input.clear();
                        app.input_mode = InputMode::Chat;
                        app.input = command;
                        let _ = app.submit_message();
                    }
                }
                KeyCode::Up => {
                    app.palette.selected = app.palette.selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    let count = app.palette_matches().len();
                    if count > 0 {
                        app.palette.selected = (app.palette.selected + 1).min(count - 1);
                    }
                }
                KeyCode::Char(c) => {
                    app.palette.input.push(c);
                    app.palette.selected = 0;
                }
                KeyCode::Backspace => {
                    app.palette.input.pop();
                    app.palette.selected = 0;
                }
                _ => {}
            }
            return Ok(());
        }
        InputMode::Project => {
            if key.code == KeyCode::Esc {
                app.project.show = false;
                app.project.content = None;
                app.input_mode = InputMode::Chat;
            }
            return Ok(());
        }
        InputMode::Board => {
            match key.code {
                KeyCode::Esc => {
                    if app.board.detail.is_some() {
                        app.board.detail = None;
                    } else {
                        app.board.show = false;
                        app.board.stories.clear();
                        app.input_mode = InputMode::Chat;
                    }
                }
                KeyCode::Enter => {
                    if app.board.detail.is_none() {
                        app.board.detail = Some(app.board.selected);
                    }
                }
                KeyCode::Up => {
                    app.board.selected = app.board.selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    let visible = super::board_view::visible_stories(&app.board.stories);
                    if !visible.is_empty() {
                        app.board.selected = (app.board.selected + 1).min(visible.len() - 1);
                    }
                }
                _ => {}
            }
            return Ok(());
        }
        InputMode::Sidebar | InputMode::Chat => {}
    }
    // Ctrl+K opens the command palette.
    if key.code == KeyCode::Char('k') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.palette.show = !app.palette.show;
        app.palette.input.clear();
        app.palette.selected = 0;
        app.input_mode = if app.palette.show {
            InputMode::Palette
        } else {
            InputMode::Chat
        };
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
    if app.sidebar.focused {
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
            app.display.show_timestamps = !app.display.show_timestamps;
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
        // Ctrl+K is handled earlier (opens command palette, line ~258).
        // kill_to_end is unreachable while palette binding exists.
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
            app.display.show_help = true;
            app.input_mode = InputMode::Help;
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
            if app.scroll.offset > 0 {
                app.scroll.offset = 0;
                app.scroll.unread_count = 0;
            } else {
                app.clear_input();
            }
        }
        _ => {}
    }
    Ok(())
}
