use std::sync::Arc;

use crossterm::event::{self, EventStream};
use futures::StreamExt;
use tokio::sync::{broadcast, mpsc};

use crate::reload;
use crate::session;
use crate::terminal::app::App;
use crate::terminal::event_dispatch::handle_terminal_event;
use crate::terminal::p2p;
use crate::terminal::ui;
use crate::transport::peer::P2pResult;
use crate::transport::webrtc;

const NOTIFICATION_PREVIEW_CHARS: usize = 50;

pub async fn run_tui<B>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut App,
    session: &session::Session,
    wake_rx: &mut broadcast::Receiver<session::Message>,
    screen_capture: Arc<std::sync::Mutex<String>>,
    mut injected_rx: mpsc::Receiver<crossterm::event::Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    B: ratatui::backend::Backend,
    B::Error: Send + Sync + 'static,
{
    let mut events = EventStream::new();
    let (p2p_tx, mut p2p_rx) = mpsc::channel::<P2pResult>(8);
    let mut active_passphrase: Option<String> = None;
    let mut pending_offer: Option<webrtc::PendingOffer> = None;
    let session_arc = Arc::new(session.clone());

    // Watch for binary changes (hot reload)
    let mut reload_rx = reload::spawn_watcher();
    let mut reload_at: Option<tokio::time::Instant> = None;

    // Periodic project state refresh
    let (project_tx, mut project_rx) = mpsc::channel::<super::board::ProjectState>(1);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Some(state) = tokio::task::spawn_blocking(super::board::fetch_project_state)
                .await
                .ok()
                .flatten()
                && project_tx.send(state).await.is_err()
            {
                break;
            }
        }
    });

    loop {
        let completed = terminal.draw(|frame| ui::render(frame, app))?;
        {
            let buffer = &completed.buffer;
            let mut capture = String::new();
            for y in 0..buffer.area.height {
                for x in 0..buffer.area.width {
                    if let Some(cell) = buffer.cell((x, y)) {
                        capture.push_str(cell.symbol());
                    }
                }
                capture.push('\n');
            }
            if let Ok(mut sc) = screen_capture.lock() {
                *sc = capture;
            }
        }
        tokio::select! {
            maybe_event = events.next() => {
                let Some(Ok(ev)) = maybe_event else { return Ok(()); };
                handle_terminal_event(ev, app, session)?;
                while event::poll(std::time::Duration::ZERO)? {
                    let ev = event::read()?;
                    handle_terminal_event(ev, app, session)?;
                }
                if app.should_quit {
                    super::ui_state::UiState::save(app, session);
                    return Ok(());
                }
                // Check for P2P commands after handling input
                if let Some(cmd) = app.p2p_command.take() {
                    p2p::handle_p2p_command(
                        cmd,
                        &p2p_tx,
                        &mut active_passphrase,
                        &mut pending_offer,
                    );
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
                            Err(e) => {
                                crate::session::diag::log_diag(&format!(
                                    "event: error reading messages: {e}"
                                ));
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Ok(());
                    }
                }
            }
            Ok(()) = reload_rx.changed() => {
                if *reload_rx.borrow() && reload_at.is_none() {
                    app.system_message("New build detected — reloading in 3 seconds...".to_string());
                    reload_at = Some(tokio::time::Instant::now() + std::time::Duration::from_secs(3));
                }
            }
            _ = async {
                if let Some(at) = reload_at {
                    tokio::time::sleep_until(at).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                super::ui_state::UiState::save(app, session);
                app.should_reload = true;
                return Ok(());
            }
            Some(state) = project_rx.recv() => {
                app.project_state = Some(state);
            }
            Some(injected) = injected_rx.recv() => {
                handle_terminal_event(injected, app, session)?;
            }
            Some(result) = p2p_rx.recv() => {
                p2p::handle_p2p_result(
                    result,
                    app,
                    &session_arc,
                    &mut active_passphrase,
                    &mut pending_offer,
                );
            }
        }
    }
}

fn handle_broadcast_message(msg: session::Message, app: &mut App) {
    let self_identity = format!("{}:human", app.peer_name);
    if msg.from != self_identity {
        let short_name = msg.from.split(':').next().unwrap_or(&msg.from).to_string();
        let preview: String = msg.text.chars().take(NOTIFICATION_PREVIEW_CHARS).collect();
        let is_dm = msg.to.as_ref().is_some_and(|r| {
            r.iter().any(|n| n == &app.peer_name) && !r.iter().all(|n| n == "all")
        });
        app.receive_message(msg);
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
