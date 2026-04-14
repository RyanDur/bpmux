use std::sync::Arc;

use crossterm::event::{self, Event, EventStream};
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

/// Pull the next terminal event from the real tty reader, or block forever
/// if the stream is `None` (headless mode). Returning a never-resolving
/// future in headless is how we keep the `tokio::select!` shape uniform
/// without creating a crossterm `EventStream` that panics with
/// "reader source not set" when there's no tty.
async fn next_terminal_event(
    stream: &mut Option<EventStream>,
) -> Option<Result<Event, std::io::Error>> {
    match stream {
        Some(s) => s.next().await,
        None => std::future::pending().await,
    }
}

pub async fn run_tui<B>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut App,
    session: &session::Session,
    wake_rx: &mut broadcast::Receiver<session::Message>,
    screen_capture: Arc<std::sync::Mutex<String>>,
    mut injected_rx: mpsc::Receiver<crossterm::event::Event>,
    headless: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    B: ratatui::backend::Backend,
    B::Error: Send + Sync + 'static,
{
    // Skip crossterm's thread-local reader setup when there's no tty.
    // `EventStream::new()` (or its first poll) panics with "reader source
    // not set" in headless contexts because there's nothing to read from.
    let mut events: Option<EventStream> = if headless {
        None
    } else {
        Some(EventStream::new())
    };
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

    // Periodic tick so stall-detection in the status bar re-renders even
    // when nothing else is happening. Without this, a peer that goes stale
    // during an idle moment would stay the wrong color until the next
    // broadcast, key press, or other event.
    let mut stall_tick = tokio::time::interval(std::time::Duration::from_secs(5));
    stall_tick.tick().await;

    // Background CI poller (#151). Skip in headless mode because there's no
    // value in polling `gh` during automated tests, and the subprocess
    // would slow test startup.
    //
    // #106: pass `session_arc` so the poller can broadcast activity-type
    // messages on state transitions (failures always; recoveries after
    // failures). Headless mode skips the poller entirely, so no broadcast
    // pathway runs during tests — transition logic is covered by pure
    // unit tests on `compute_broadcast_text` in `ci.rs`.
    let (ci_tx, mut ci_rx) = mpsc::channel::<super::ci::CiState>(4);
    if !headless {
        super::ci::spawn_poller(ci_tx, Some(Arc::clone(&session_arc)));
    }

    // Perf: throttle screen capture in interactive mode to avoid
    // per-keystroke string allocation. In headless mode, always
    // capture so desk-checks via capture_screen get current data.
    let mut capture_countdown: u32 = 0;
    let capture_interval: u32 = if headless { 1 } else { 30 };

    loop {
        let completed = terminal.draw(|frame| ui::render(frame, app))?;
        capture_countdown = capture_countdown.saturating_sub(1);
        if capture_countdown == 0 {
            capture_countdown = capture_interval;
            let buffer = &completed.buffer;
            let mut capture = String::with_capacity(
                (buffer.area.width as usize + 1) * buffer.area.height as usize,
            );
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
            maybe_event = next_terminal_event(&mut events) => {
                let Some(Ok(ev)) = maybe_event else { return Ok(()); };
                handle_terminal_event(ev, app, session)?;
                // Drain any additional queued events without blocking. This
                // is only meaningful when a real tty is attached; in headless
                // mode `events` is None and we never reach this branch.
                if !headless {
                    while event::poll(std::time::Duration::ZERO)? {
                        let ev = event::read()?;
                        handle_terminal_event(ev, app, session)?;
                    }
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
                                // Rebuild classification cache after lag recovery
                                app.class_cache = messages.iter().map(|m| {
                                    if m.is_chat() {
                                        Some(crate::session::classify(m, &app.peer_name))
                                    } else {
                                        None
                                    }
                                }).collect();
                                app.messages = messages;
                                app.push_message(session::Message {
                                    from: "bpmux".to_string(),
                                    text: format!("Caught up — {n} messages arrived while processing."),
                                    ts: chrono::Utc::now(),
                                    message_type: crate::session::MessageType::System,
                                    to: None,
                                    addressee: None,
                                    multi_depth: None,
                                    topic: None,
                                    explicit_privacy: false,
                                });
                                app.scroll.offset = 0;
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
            _ = stall_tick.tick() => {
                // Intentionally empty — the tick exists only so the loop
                // iterates and the top-of-loop `terminal.draw(...)` re-runs
                // the stall-detection logic in the status bar.
            }
            Some(ci) = ci_rx.recv() => {
                app.ci_state = ci;
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
