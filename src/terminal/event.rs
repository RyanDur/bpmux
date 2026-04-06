use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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

/// Returns true when an incoming message is specifically directed at `peer_name`
/// (i.e. `msg.to` contains the peer) and is NOT a plain broadcast.
fn should_bell(msg: &session::Message, peer_name: &str) -> bool {
    match &msg.to {
        None => false,
        Some(recipients) => {
            recipients.iter().any(|r| r == peer_name) && !recipients.iter().all(|r| r == "all")
        }
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
                    if should_bell(&msg, &app.peer_name) {
                        bell();
                    }
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
            let Event::Key(key) = ev else {
                break;
            };
            if key.kind != KeyEventKind::Press {
                if !event::poll(std::time::Duration::ZERO)? {
                    break;
                }
                continue;
            }
            match key.code {
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
                    app.tab_complete();
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn msg(to: Option<Vec<&str>>) -> session::Message {
        session::Message {
            from: "pm:claude-code".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: to.map(|v| v.into_iter().map(String::from).collect()),
        }
    }

    #[test]
    fn broadcast_does_not_bell() {
        assert!(!should_bell(&msg(None), "ryan"));
    }

    #[test]
    fn directed_to_peer_bells() {
        assert!(should_bell(&msg(Some(vec!["ryan"])), "ryan"));
    }

    #[test]
    fn directed_to_other_does_not_bell() {
        assert!(!should_bell(&msg(Some(vec!["pm"])), "ryan"));
    }

    #[test]
    fn all_does_not_bell() {
        assert!(!should_bell(&msg(Some(vec!["all"])), "ryan"));
    }

    #[test]
    fn directed_to_peer_among_multiple_bells() {
        assert!(should_bell(&msg(Some(vec!["ryan", "pm"])), "ryan"));
    }

    #[test]
    fn directed_to_others_not_peer_does_not_bell() {
        assert!(!should_bell(&msg(Some(vec!["pm", "designer"])), "ryan"));
    }
}
