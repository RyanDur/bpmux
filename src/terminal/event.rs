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
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Enter => {
                    if let Some(msg) = app.submit_message() {
                        session.broadcast(&msg)?;
                    }
                    if app.should_quit {
                        return Ok(());
                    }
                }
                KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.show_timestamps = !app.show_timestamps;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.scroll_up(10);
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let before = app.scroll_offset;
                    app.scroll_down(10);
                    if app.scroll_offset == before {
                        bell();
                    }
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if app.input.is_empty() {
                        return Ok(());
                    }
                    app.input.clear();
                }
                KeyCode::Up if key.modifiers.is_empty() => {
                    app.history_up();
                }
                KeyCode::Down if key.modifiers.is_empty() => {
                    app.history_down();
                }
                KeyCode::Char(c) => {
                    app.input.push(c);
                }
                KeyCode::Backspace => {
                    app.input.pop();
                }
                KeyCode::PageUp => {
                    app.scroll_up(5);
                }
                KeyCode::PageDown => {
                    let before = app.scroll_offset;
                    app.scroll_down(5);
                    if app.scroll_offset == before {
                        bell();
                    }
                }
                KeyCode::Esc => {
                    app.input.clear();
                }
                _ => {}
            }
        }
    }
}
