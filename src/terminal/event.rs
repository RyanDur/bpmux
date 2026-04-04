use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use tokio::sync::broadcast;

use crate::session;
use crate::terminal::app::App;
use crate::terminal::ui;

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
                    let scrolled_up = app.scroll_offset > 0;
                    app.receive_message(msg);
                    if scrolled_up || app.scroll_offset > 0 {
                        use std::io::Write;
                        if let Ok(mut tty) =
                            std::fs::OpenOptions::new().write(true).open("/dev/tty")
                        {
                            let _ = tty.write_all(b"\x07");
                            let _ = tty.flush();
                        }
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
                    app.scroll_down(10);
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
                    app.scroll_down(5);
                }
                KeyCode::Esc => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}
