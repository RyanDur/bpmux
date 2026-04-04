use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use tokio::sync::broadcast;

use crate::session;
use crate::terminal::app::App;
use crate::terminal::ui;

pub fn run_tui(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    session_dir: &Path,
    wake_rx: &mut broadcast::Receiver<session::Message>,
    wake_tx: &broadcast::Sender<session::Message>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        // Check for messages from other sources (MCP, WebSocket) via broadcast
        match wake_rx.try_recv() {
            Ok(msg) => {
                // Skip messages we sent from this TUI (already in app.messages via submit_message)
                let self_identity = session::participant_id(&app.peer_name, session::ROLE_HUMAN);
                if msg.from != self_identity {
                    app.messages.push(msg);
                }
                continue; // Redraw immediately
            }
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                // Fell behind — reload everything from disk
                match session::read_messages(session_dir) {
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
                        session::broadcast_message(session_dir, &msg, wake_tx)?;
                    }
                    if app.should_quit {
                        return Ok(());
                    }
                }
                KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.show_timestamps = !app.show_timestamps;
                }
                KeyCode::Char(c) => {
                    app.input.push(c);
                }
                KeyCode::Backspace => {
                    app.input.pop();
                }
                KeyCode::Esc => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}
