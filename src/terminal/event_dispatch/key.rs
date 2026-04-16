use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::session;
use crate::terminal::app::{App, InputMode};

const PAGE_SCROLL_LINES: usize = 10;

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

pub(super) fn handle(
    key: KeyEvent,
    app: &mut App,
    session: &session::Session,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Modal dispatch via InputMode enum — explicit state machine
    // replacing the old implicit priority chain of boolean flags.
    match app.input_mode {
        InputMode::Help => {
            app.input_mode = InputMode::Chat;
            return Ok(());
        }
        InputMode::Tokens => {
            app.tokens.rows.clear();
            app.input_mode = InputMode::Chat;
            return Ok(());
        }
        InputMode::Palette => {
            match key.code {
                KeyCode::Esc => {
                    app.palette.input.clear();
                    app.input_mode = InputMode::Chat;
                }
                KeyCode::Enter => {
                    let matches = app.palette_matches();
                    if let Some((cmd, _)) = matches.get(app.palette.selected) {
                        let command = cmd.to_string();
                        app.palette.input.clear();
                        app.input_mode = InputMode::Chat;
                        app.editor.input = command;
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
                        app.board.stories.clear();
                        app.input_mode = InputMode::Chat;
                    }
                }
                KeyCode::Enter if app.board.detail.is_none() => {
                    app.board.detail = Some(app.board.selected);
                }
                KeyCode::Up => {
                    app.board.selected = app.board.selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    let visible = crate::terminal::board_view::visible_stories(&app.board.stories);
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
        app.palette.input.clear();
        app.palette.selected = 0;
        app.input_mode = if matches!(app.input_mode, InputMode::Palette) {
            InputMode::Chat
        } else {
            InputMode::Palette
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
        KeyCode::Tab if popup_open && app.editor.popup_selected.is_some() => {
            app.popup_accept();
        }
        KeyCode::Enter if popup_open && app.editor.popup_selected.is_some() => {
            app.popup_accept();
        }
        KeyCode::Esc if popup_open => {
            app.popup_dismiss();
            app.clear_input();
        }
        KeyCode::Enter => {
            if app.editor.input.is_empty() {
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
            if app.editor.input.is_empty() {
                app.lifecycle.should_quit = true;
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
        // Ctrl+K is handled earlier (opens command palette).
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
        KeyCode::Char('?') if app.editor.input.is_empty() => {
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
