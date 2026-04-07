use crossterm::event::{
    self, Event, EventStream, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
};
use futures::StreamExt;
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

fn scroll_up_with_bell(app: &mut App, amount: usize) {
    let before = app.scroll_offset;
    app.scroll_up(amount);
    if app.scroll_offset == before && before > 0 && !app.at_boundary {
        bell();
        app.at_boundary = true;
    }
}

fn scroll_down_with_bell(app: &mut App, amount: usize) {
    let before = app.scroll_offset;
    app.scroll_down(amount);
    if app.scroll_offset == before && !app.at_boundary {
        bell();
        app.at_boundary = true;
    }
}

pub async fn run_tui(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    session: &session::Session,
    wake_rx: &mut broadcast::Receiver<session::Message>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut events = EventStream::new();
    loop {
        app.connected_peers = session.peer_count();
        terminal.draw(|frame| ui::render(frame, app))?;
        tokio::select! {
            maybe_event = events.next() => {
                let Some(Ok(ev)) = maybe_event else { return Ok(()); };
                handle_terminal_event(ev, app, session)?;
                // Drain any buffered events before re-rendering
                while event::poll(std::time::Duration::ZERO)? {
                    let ev = event::read()?;
                    handle_terminal_event(ev, app, session)?;
                }
                if app.should_quit {
                    return Ok(());
                }
            }
            result = wake_rx.recv() => {
                match result {
                    Ok(msg) => {
                        handle_broadcast_message(msg, app);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        match session.read_messages() {
                            Ok(messages) => {
                                app.messages = messages;
                                app.messages.push(session::Message {
                                    from: "bpmux".to_string(),
                                    text: format!("Caught up — {n} messages arrived while processing."),
                                    ts: chrono::Utc::now(),
                                    message_type: crate::session::MessageType::System,
                                    to: None,
                                });
                                app.scroll_offset = 0;
                            }
                            Err(e) => eprintln!("Error reading messages: {e}"),
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn handle_broadcast_message(msg: session::Message, app: &mut App) {
    let self_identity = format!("{}:human", app.peer_name);
    if msg.from != self_identity {
        let short_name = msg.from.split(':').next().unwrap_or(&msg.from).to_string();
        let preview: String = msg.text.chars().take(50).collect();
        let is_dm = msg.to.as_ref().is_some_and(|r| {
            r.iter().any(|n| n == &app.peer_name) && !r.iter().all(|n| n == "all")
        });
        app.receive_message(msg);
        // OS notification when TUI is unfocused
        if !app.tui_focused {
            let safe_preview = preview.replace('\\', "\\\\").replace('"', "\\\"");
            let safe_name = short_name.replace('\\', "\\\\").replace('"', "\\\"");
            let sound = if is_dm { " sound name \"default\"" } else { "" };
            let _ = std::process::Command::new("osascript")
                .args([
                    "-e",
                    &format!(
                        "display notification \"{}\" with title \"bpmux — {}\"{}",
                        safe_preview, safe_name, sound
                    ),
                ])
                .spawn();
        }
    }
}

fn handle_mouse_click(col: u16, row: u16, app: &mut App) {
    // Check if click is in the sidebar area
    if let Some((sx, sy, sw, _sh)) = app.sidebar_area {
        if col >= sx && col < sx + sw && row >= sy {
            // Inside sidebar — find which channel was clicked
            // Border top = row sy, so content starts at sy + 1
            let content_row = row.saturating_sub(sy + 1);
            let channels = app.sidebar_channels();
            let idx = content_row as usize;
            if idx < channels.len() {
                app.sidebar_selected = idx;
                app.sidebar_select();
            }
            return;
        }
    } else if col < 3 {
        // Sidebar is closed — clicking the left edge opens it
        app.toggle_sidebar();
        return;
    }

    // Check if click is on "? help" in the status bar
    if let Some((_, sb_y, _, _)) = app.status_bar_area
        && row == sb_y
        && app.help_hint_col.is_some_and(|hint_col| col >= hint_col)
    {
        app.show_help = true;
    }
}

fn handle_terminal_event(
    ev: Event,
    app: &mut App,
    session: &session::Session,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Track terminal focus for notification gating
    if matches!(&ev, Event::FocusGained) {
        app.tui_focused = true;
        return Ok(());
    }
    if matches!(&ev, Event::FocusLost) {
        app.tui_focused = false;
        return Ok(());
    }
    // Handle mouse events
    if let Event::Mouse(mouse) = &ev {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.scroll_up(3);
            }
            MouseEventKind::ScrollDown => {
                app.scroll_down(3);
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                handle_mouse_click(mouse.column, mouse.row, app);
            }
            _ => {}
        }
        return Ok(());
    }
    // Bracketed paste: insert all pasted text (including newlines) into the buffer
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
    // Help overlay — any key dismisses it
    if app.show_help {
        app.show_help = false;
        return Ok(());
    }
    // Sidebar toggle: Ctrl+B, Ctrl+1, or Alt+1 (IntelliJ project tool window)
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
    // When sidebar is focused, intercept navigation keys
    if app.sidebar_focused {
        match key.code {
            KeyCode::Up => app.sidebar_up(),
            KeyCode::Down => app.sidebar_down(),
            KeyCode::Left => app.sidebar_collapse(),
            KeyCode::Right => app.sidebar_expand(),
            KeyCode::Enter => app.sidebar_select(),
            KeyCode::Esc => app.sidebar_close(),
            _ => {}
        }
        return Ok(());
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
            }
        }
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.show_timestamps = !app.show_timestamps;
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            scroll_up_with_bell(app, 10);
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            scroll_down_with_bell(app, 10);
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
            scroll_up_with_bell(app, 5);
        }
        KeyCode::PageDown => {
            scroll_down_with_bell(app, 5);
        }
        KeyCode::Esc => {
            app.clear_input();
        }
        _ => {}
    }
    Ok(())
}
