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
    if (key.code == KeyCode::Char('p') || key.code == KeyCode::Char('2'))
        && key.modifiers.contains(KeyModifiers::ALT)
    {
        app.toggle_board_panel();
        return Ok(());
    }
    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.toggle_board_panel();
        return Ok(());
    }
    if app.board.panel_focused {
        let handled = match key.code {
            KeyCode::Up if app.board.panel_detail.is_some() => {
                app.panel_detail_scroll_up(1);
                true
            }
            KeyCode::Down if app.board.panel_detail.is_some() => {
                app.panel_detail_scroll_down(1);
                true
            }
            KeyCode::PageUp if app.board.panel_detail.is_some() => {
                app.panel_detail_scroll_up(PAGE_SCROLL_LINES as u16);
                true
            }
            KeyCode::PageDown if app.board.panel_detail.is_some() => {
                app.panel_detail_scroll_down(PAGE_SCROLL_LINES as u16);
                true
            }
            KeyCode::Char('u')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && app.board.panel_detail.is_some() =>
            {
                app.panel_detail_scroll_up(PAGE_SCROLL_LINES as u16);
                true
            }
            KeyCode::Char('d')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && app.board.panel_detail.is_some() =>
            {
                app.panel_detail_scroll_down(PAGE_SCROLL_LINES as u16);
                true
            }
            KeyCode::Up => {
                app.panel_select_up();
                true
            }
            KeyCode::Down => {
                app.panel_select_down();
                true
            }
            KeyCode::Enter if app.editor.input.is_empty() => {
                app.panel_open_detail();
                true
            }
            KeyCode::Esc => {
                if app.board.panel_detail.is_some() {
                    app.panel_close_detail();
                } else {
                    app.board.panel_focused = false;
                }
                true
            }
            _ => false,
        };
        if handled {
            return Ok(());
        }
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

    #[test]
    fn ctrl_p_toggles_board_panel() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();
        let before = app.board.panel_open;

        let ctrl_p = KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(ctrl_p, &mut app, &session).unwrap();

        assert_ne!(app.board.panel_open, before);
    }

    #[test]
    fn alt_p_toggles_board_panel() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();
        let before = app.board.panel_open;

        let alt_p = KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(alt_p, &mut app, &session).unwrap();

        assert_ne!(app.board.panel_open, before);
    }

    #[test]
    fn board_panel_focused_char_keys_fall_through_to_chat() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.board.panel_focused = true;
        let (session, _dir) = test_session();

        for c in "hello".chars() {
            handle(char_key(c), &mut app, &session).unwrap();
        }

        assert_eq!(app.editor.input, "hello");
        assert!(app.board.panel_focused);
    }

    #[test]
    fn board_panel_focused_arrow_keys_navigate() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.board.panel_focused = true;
        let (session, _dir) = test_session();

        let down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(down, &mut app, &session).unwrap();

        assert!(
            app.editor.input.is_empty(),
            "Down must not reach chat input"
        );
    }

    #[test]
    fn board_panel_focused_enter_falls_through_when_input_nonempty() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.board.panel_focused = true;
        let (session, _dir) = test_session();

        for c in "hi".chars() {
            handle(char_key(c), &mut app, &session).unwrap();
        }
        assert_eq!(app.editor.input, "hi");

        let enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(enter, &mut app, &session).unwrap();

        assert!(
            app.board.panel_detail.is_none(),
            "Enter with input must not drill in"
        );
        assert!(
            app.editor.input.is_empty(),
            "Enter with input must submit + clear"
        );
    }

    #[test]
    fn board_panel_focused_enter_opens_detail_when_input_empty() {
        use crate::terminal::board::{BoardStory, StoryState};
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.board.panel_focused = true;
        app.board.stories = vec![BoardStory {
            number: 1,
            title: "test".to_string(),
            state: StoryState::Ready,
            body: "body".to_string(),
            assignees: vec![],
            created: "2026-04-19".to_string(),
            reason: String::new(),
            labels: vec!["ready".to_string()],
        }];
        let (session, _dir) = test_session();

        let enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(enter, &mut app, &session).unwrap();

        assert!(
            app.board.panel_detail.is_some(),
            "Enter with empty input must drill in"
        );
    }

    #[test]
    fn question_mark_opens_help_when_input_empty() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();
        handle(char_key('?'), &mut app, &session).unwrap();
        assert!(matches!(app.input_mode, InputMode::Help));
    }

    #[test]
    fn question_mark_inserts_char_when_input_nonempty() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.insert_char('h');
        app.insert_char('i');
        let (session, _dir) = test_session();
        handle(char_key('?'), &mut app, &session).unwrap();
        assert_eq!(app.editor.input, "hi?");
        assert!(matches!(app.input_mode, InputMode::Chat));
    }

    #[test]
    fn alt_1_toggles_sidebar() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();
        let before = app.sidebar.open;
        let alt_1 = KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(alt_1, &mut app, &session).unwrap();
        assert_ne!(app.sidebar.open, before);
    }

    #[test]
    fn plain_1_routes_to_input_not_sidebar() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();
        let before = app.sidebar.open;
        handle(char_key('1'), &mut app, &session).unwrap();
        assert_eq!(app.sidebar.open, before);
        assert_eq!(app.editor.input, "1");
    }

    #[test]
    fn ctrl_b_toggles_sidebar() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();
        let before = app.sidebar.open;
        let ctrl_b = KeyEvent {
            code: KeyCode::Char('b'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(ctrl_b, &mut app, &session).unwrap();
        assert_ne!(app.sidebar.open, before);
    }

    #[test]
    fn ctrl_1_toggles_sidebar() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();
        let before = app.sidebar.open;
        let ctrl_1 = KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(ctrl_1, &mut app, &session).unwrap();
        assert_ne!(app.sidebar.open, before);
    }

    #[test]
    fn esc_in_help_mode_returns_to_chat() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input_mode = InputMode::Help;
        let (session, _dir) = test_session();
        let esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(esc, &mut app, &session).unwrap();
        assert!(matches!(app.input_mode, InputMode::Chat));
    }

    #[test]
    fn ctrl_k_enters_palette_mode() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        let (session, _dir) = test_session();
        let ctrl_k = KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(ctrl_k, &mut app, &session).unwrap();
        assert!(matches!(app.input_mode, InputMode::Palette));
    }

    #[test]
    fn palette_char_input_filters_matches() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input_mode = InputMode::Palette;
        let (session, _dir) = test_session();
        for c in "who".chars() {
            handle(char_key(c), &mut app, &session).unwrap();
        }
        assert_eq!(app.palette.input, "who");
        let names: Vec<&str> = app.palette_matches().iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"/who"));
        assert!(!names.contains(&"/help"));
    }

    #[test]
    fn palette_esc_clears_and_exits_to_chat() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input_mode = InputMode::Palette;
        app.palette.input = "who".to_string();
        let (session, _dir) = test_session();
        let esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(esc, &mut app, &session).unwrap();
        assert!(matches!(app.input_mode, InputMode::Chat));
        assert!(app.palette.input.is_empty());
    }

    #[test]
    fn palette_up_down_moves_selection() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input_mode = InputMode::Palette;
        let (session, _dir) = test_session();
        let down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(down, &mut app, &session).unwrap();
        assert_eq!(app.palette.selected, 1);
        let up = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(up, &mut app, &session).unwrap();
        assert_eq!(app.palette.selected, 0);
    }

    #[test]
    fn palette_enter_exits_and_runs_selected() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input_mode = InputMode::Palette;
        app.palette.input = "who".to_string();
        let (session, _dir) = test_session();
        let enter = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(enter, &mut app, &session).unwrap();
        assert!(matches!(app.input_mode, InputMode::Chat));
        assert!(app.palette.input.is_empty());
    }

    #[test]
    fn palette_backspace_removes_char() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.input_mode = InputMode::Palette;
        app.palette.input = "who".to_string();
        let (session, _dir) = test_session();
        let bs = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(bs, &mut app, &session).unwrap();
        assert_eq!(app.palette.input, "wh");
    }

    #[test]
    fn board_panel_focused_esc_unfocuses() {
        let mut app = App::new("ryan".to_string(), "test".to_string(), vec![]);
        app.board.panel_focused = true;
        let (session, _dir) = test_session();

        let esc = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        handle(esc, &mut app, &session).unwrap();

        assert!(!app.board.panel_focused);
    }
}
