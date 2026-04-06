use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::DefaultTerminal;
use tokio::sync::broadcast;

use crate::session;
use crate::terminal::app::App;
use crate::terminal::ui;

fn bell() {
    use std::io::Write;
    if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        let _ = tty.write_all(b"\x07");
        let _ = tty.flush();
    }
}

pub fn run_tui(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    session: &session::Session,
    wake_rx: &mut broadcast::Receiver<session::Message>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        app.connected_peers = session.peer_count();
        terminal.draw(|frame| ui::render(frame, app))?;
        match wake_rx.try_recv() {
            Ok(msg) => {
                let self_identity = format!("{}:human", app.peer_name);
                if msg.from != self_identity {
                    let short_name = msg.from.split(':').next().unwrap_or(&msg.from).to_string();
                    let preview: String = msg.text.chars().take(50).collect();
                    let was_scrolled = app.scroll_offset > 0;
                    app.receive_message(msg);
                    if was_scrolled || app.scroll_offset > 0 {
                        let _ = std::process::Command::new("osascript")
                            .args([
                                "-e",
                                &format!(
                                    "display notification \"{}\" with title \"bpmux — {}\"",
                                    preview, short_name
                                ),
                            ])
                            .spawn();
                    }
                }
                continue;
            }
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                match session.read_messages() {
                    Ok(messages) => app.messages = messages,
                    Err(e) => eprintln!("Error reading messages: {e}"),
                }
                continue;
            }
            Err(broadcast::error::TryRecvError::Empty) => {}
            Err(broadcast::error::TryRecvError::Closed) => {
                return Ok(());
            }
        }
        // Drain ALL pending key events before re-rendering.
        // Prevents buffer lag when key repeat is faster than render.
        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }
        loop {
            let ev = event::read()?;
            // Handle mouse scroll events
            if let Event::Mouse(mouse) = &ev {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        app.scroll_up(3);
                    }
                    MouseEventKind::ScrollDown => {
                        app.scroll_down(3);
                    }
                    _ => {}
                }
                if !event::poll(std::time::Duration::ZERO)? {
                    break;
                }
                continue;
            }
            let Event::Key(key) = ev else {
                break;
            };
            if key.kind != KeyEventKind::Press {
                if !event::poll(std::time::Duration::ZERO)? {
                    break;
                }
                continue;
            }
            // Help overlay — any key dismisses it
            if app.show_help {
                app.show_help = false;
                if !event::poll(std::time::Duration::ZERO)? {
                    break;
                }
                continue;
            }
            // When popup is visible, intercept navigation keys
            let popup_open = app.popup_visible();
            match key.code {
                // Popup navigation — Up/Down move selection, Tab/Enter accept
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
                    } else {
                        if let Some(msg) = app.submit_message() {
                            session.broadcast(&msg)?;
                        }
                        if app.should_quit {
                            return Ok(());
                        }
                    }
                }
                KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.show_timestamps = !app.show_timestamps;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let before = app.scroll_offset;
                    app.scroll_up(10);
                    if app.scroll_offset == before && before > 0 && !app.at_boundary {
                        bell();
                        app.at_boundary = true;
                    }
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let before = app.scroll_offset;
                    app.scroll_down(10);
                    if app.scroll_offset == before && !app.at_boundary {
                        bell();
                        app.at_boundary = true;
                    }
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if app.input.is_empty() {
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
                        // Start popup selection at first item
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
                    let before = app.scroll_offset;
                    app.scroll_up(5);
                    if app.scroll_offset == before && before > 0 && !app.at_boundary {
                        bell();
                        app.at_boundary = true;
                    }
                }
                KeyCode::PageDown => {
                    let before = app.scroll_offset;
                    app.scroll_down(5);
                    if app.scroll_offset == before && !app.at_boundary {
                        bell();
                        app.at_boundary = true;
                    }
                }
                KeyCode::Esc => {
                    app.clear_input();
                }
                _ => {}
            }
            // Check for more buffered events
            if !event::poll(std::time::Duration::ZERO)? {
                break;
            }
        }
    }
}
