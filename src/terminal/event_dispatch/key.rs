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
            // Esc closes the project overlay. Other keys fall through
            // to the shared Chat dispatcher below (#217): the project
            // panel is a visual overlay, not a keyboard-capturing
            // modal, so typing must continue to reach the chat input
            // while the split view is open.
            if key.code == KeyCode::Esc {
                app.project.content = None;
                app.input_mode = InputMode::Chat;
                return Ok(());
            }
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::session;
    use crossterm::event::KeyEventKind;
    use tempfile::TempDir;
    use tokio::sync::broadcast;

    /// Build a minimal Session for dispatch tests. The Session is
    /// unused unless the test triggers Enter-submit (which broadcasts
    /// a message), so a temp dir + throwaway broadcast channel is
    /// enough for typing-only assertions.
    fn test_session() -> (session::Session, TempDir) {
        let dir = TempDir::new().unwrap();
        let (tx, _rx) = broadcast::channel(16);
        let session = session::Session::new(dir.path().to_path_buf(), "ryan".to_string(), tx);
        (session, dir)
    }

    fn char_key(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    /// #217: typing while the project panel is open must reach the
    /// chat input — the panel is a visual overlay, not a modal that
    /// captures keyboard.
    #[test]
    fn project_mode_falls_through_to_chat_for_char_keys() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input_mode = InputMode::Project;
        let (session, _dir) = test_session();

        for c in "hello".chars() {
            handle(char_key(c), &mut app, &session).unwrap();
        }

        assert_eq!(
            app.editor.input, "hello",
            "project mode must not swallow typed chars"
        );
        assert!(
            matches!(app.input_mode, InputMode::Project),
            "typing should not exit project mode"
        );
    }

    /// Esc in project mode still closes the overlay — the fall-
    /// through must not regress the legitimate modal exit path.
    #[test]
    fn project_mode_still_closes_on_esc() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input_mode = InputMode::Project;
        app.project.content = Some("board".to_string());
        let (session, _dir) = test_session();

        let esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(esc, &mut app, &session).unwrap();

        assert!(matches!(app.input_mode, InputMode::Chat));
        assert!(app.project.content.is_none());
    }

    /// Opening + closing the overlay repeatedly does not accumulate
    /// stuck state (the original #217 failure mode: input locked
    /// progressively worse with each re-entry).
    #[test]
    fn project_mode_typing_survives_repeated_open_close() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();

        for _ in 0..5 {
            app.input_mode = InputMode::Project;
            handle(char_key('a'), &mut app, &session).unwrap();
            app.input_mode = InputMode::Chat;
        }

        assert_eq!(app.editor.input, "aaaaa");
    }
}
