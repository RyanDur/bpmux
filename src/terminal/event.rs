use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use tokio::sync::broadcast;

use crate::session;
use crate::terminal::app::App;
use crate::terminal::ui;

pub fn run_tui(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    session_dir: &Path,
    wake_rx: &mut broadcast::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        // Check for wake signals from MCP server (non-blocking)
        if wake_rx.try_recv().is_ok() {
            // Reload messages from the log to pick up MCP-sent messages
            match session::read_messages(session_dir) {
                Ok(messages) => app.messages = messages,
                Err(e) => eprintln!("Error reading messages: {e}"),
            }
            continue; // Redraw immediately
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
                        session::append_message(session_dir, &msg)?;
                    }
                    if app.should_quit {
                        return Ok(());
                    }
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
