use std::sync::Arc;

use crossterm::event::{
    self, Event, EventStream, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
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

/// Transport-neutral inject event. Outside callers (REST simulate
/// endpoints) send these instead of raw `crossterm::event::Event`
/// so the transport layer doesn't need to know about crossterm.
/// Terminal owns the conversion because crossterm is a terminal-
/// layer concern.
#[derive(Debug, Clone)]
pub enum TerminalInjectEvent {
    /// Mouse click at `(x, y)` — 0-based terminal cell coordinates.
    /// Always interpreted as a left-button Down event with no
    /// modifiers; the REST API doesn't expose richer mouse semantics.
    Click { x: u16, y: u16 },
    /// Key press by name plus optional modifier-name list. Name
    /// vocabulary matches the REST API's payload (e.g., `"enter"`,
    /// `"esc"`, `"a"`); modifiers are `"ctrl" | "alt" | "shift" |
    /// "super"` (case-insensitive).
    Key { key: String, modifiers: Vec<String> },
    /// Multi-char text → crossterm `Event::Paste`.
    Type { text: String },
    /// Direct `app.clear_input()` — no key sequence covers it universally
    /// (Ctrl+U scrolls; Esc only clears at bottom-scroll).
    Clear,
    /// Direct `app.submit_message()`.
    Submit,
}

/// Reasons a `TerminalInjectEvent` can fail to convert. Exposed so
/// transport handlers can surface a 400 without duplicating the
/// vocabulary-parsing logic.
#[derive(Debug, Clone)]
pub enum InjectEventError {
    UnknownKey(String),
    UnknownModifier(String),
}

impl InjectEventError {
    pub fn message(&self) -> String {
        match self {
            Self::UnknownKey(k) => format!("unknown key: {k}"),
            Self::UnknownModifier(m) => format!("unknown modifier: {m}"),
        }
    }
}

impl TerminalInjectEvent {
    /// Check that the key + modifier vocabulary is recognized
    /// without doing the actual conversion. Transport handlers call
    /// this at request-validation time so bad input surfaces as a
    /// 400 before anything enters the channel.
    pub fn validate(&self) -> Result<(), InjectEventError> {
        match self {
            Self::Click { .. } | Self::Type { .. } | Self::Clear | Self::Submit => Ok(()),
            Self::Key { key, modifiers } => {
                key_code_for(key)?;
                for name in modifiers {
                    modifier_for(name)?;
                }
                Ok(())
            }
        }
    }

    /// Convert into a crossterm `Event`. Called from the event-loop
    /// consumer after the transport boundary has already validated;
    /// errors at this point are bugs, not bad input.
    /// Convert to a crossterm `Event`. `Clear`/`Submit` are errors —
    /// event loop must dispatch them directly before calling this.
    pub fn into_crossterm(self) -> Result<Event, InjectEventError> {
        match self {
            Self::Click { x, y } => Ok(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: x,
                row: y,
                modifiers: KeyModifiers::empty(),
            })),
            Self::Key { key, modifiers } => {
                let key_code = key_code_for(&key)?;
                let mut bits = KeyModifiers::empty();
                for name in modifiers {
                    bits |= modifier_for(&name)?;
                }
                Ok(Event::Key(KeyEvent::new(key_code, bits)))
            }
            Self::Type { text } => Ok(Event::Paste(text)),
            Self::Clear | Self::Submit => Err(InjectEventError::UnknownKey(
                "Clear/Submit are bpmux actions, not keystrokes — dispatch before into_crossterm"
                    .to_string(),
            )),
        }
    }
}

fn key_code_for(key: &str) -> Result<KeyCode, InjectEventError> {
    match key {
        "enter" => Ok(KeyCode::Enter),
        "escape" | "esc" => Ok(KeyCode::Esc),
        "tab" => Ok(KeyCode::Tab),
        "backspace" => Ok(KeyCode::Backspace),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        s if s.chars().count() == 1 => Ok(KeyCode::Char(s.chars().next().unwrap_or(' '))),
        _ => Err(InjectEventError::UnknownKey(key.to_string())),
    }
}

fn modifier_for(name: &str) -> Result<KeyModifiers, InjectEventError> {
    match name.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Ok(KeyModifiers::CONTROL),
        "alt" => Ok(KeyModifiers::ALT),
        "shift" => Ok(KeyModifiers::SHIFT),
        "super" => Ok(KeyModifiers::SUPER),
        _ => Err(InjectEventError::UnknownModifier(name.to_string())),
    }
}

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
    mut injected_rx: mpsc::Receiver<TerminalInjectEvent>,
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
            // #320: was `spawn_blocking(fetch_project_state)` back when
            // the fetcher shelled out to gh. Now it's async over
            // octocrab and runs directly on the current task.
            if let Some(state) = super::board::fetch_project_state().await
                && project_tx.send(state).await.is_err()
            {
                break;
            }
        }
    });

    // Periodic DORA refresh. Separate channel so project state
    // and DORA can tick independently. Fires at startup + every 2 min
    // (DORA changes more slowly than board data).
    let (dora_tx, mut dora_rx) = mpsc::channel::<super::board::DoraMetrics>(1);
    if !headless {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
            loop {
                interval.tick().await;
                // #320: was `spawn_blocking(fetch_dora_metrics)` when the
                // fetcher was sync-gh. Now async over octocrab.
                if let Some(metrics) = super::board::fetch_dora_metrics().await
                    && dora_tx.send(metrics).await.is_err()
                {
                    break;
                }
            }
        });
    }

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
    // Pass `session_arc` so the poller can broadcast activity-type
    // messages on state transitions (failures always; recoveries after
    // failures). Headless mode skips the poller entirely, so no broadcast
    // pathway runs during tests — transition logic is covered by pure
    // unit tests on `compute_broadcast_text` in `ci.rs`.
    let (ci_tx, mut ci_rx) = mpsc::channel::<crate::ci::CiState>(4);
    if !headless {
        // #300 hydrate: scan the already-loaded message log for the most
        // recent CI broadcast so the poller's first poll doesn't re-emit
        // a phantom "CI failed" on TUI restart during a Red period. Pure
        // scan over owned data; cheap at startup.
        let hydrated = crate::ci::hydrate_last_broadcast(&app.msg.messages);
        crate::ci::spawn_poller(ci_tx, Some(Arc::clone(&session_arc)), hydrated);
    }

    // Background board poller (#219). Polls GitHub REST API for issue
    // state, writes to the shared snapshot. View reads from the
    // snapshot — no fetch on open, no loading state.
    //
    // Requires BPMUX_REPO ("owner/repo" format) and GH_TOKEN env vars.
    // Silent no-op in headless mode or when env vars are missing —
    // the existing gh-based board fallback still works for tests.
    let board_poller_tx = if !headless {
        match std::env::var("BPMUX_REPO").ok().and_then(|r| {
            let (o, rp) = r.split_once('/')?;
            Some((o.to_string(), rp.to_string()))
        }) {
            Some((owner, repo)) => Some(super::board_poller::spawn(
                Arc::clone(&app.board_snapshot),
                owner,
                repo,
            )),
            None => None,
        }
    } else {
        None
    };
    // Mirror onto App so view-open paths can trigger defensive
    // refetches (#219 re-work). Event loop still owns the primary tx
    // for signal-driven refresh below.
    app.board_poller_tx = board_poller_tx.clone();

    // On-demand board fetcher (#294 freeze-fix, #320 gh→octocrab).
    // Before #294, `request_board_refresh_if_stale` called
    // `fetch_board_stories` synchronously on the event loop thread;
    // under slow gh that blocked every `select!` arm. The fix moved
    // the fetch behind a signal channel so the listener task owns
    // the await. #320 then swapped the subprocess for octocrab —
    // async-native, so this task no longer needs `spawn_blocking`,
    // but the signal shape + 10s timeout stay for the same starve-
    // prevention guarantee (a hung HTTPS connection could otherwise
    // stall refetches indefinitely).
    let (board_fetch_request_tx, mut board_fetch_request_rx) = mpsc::channel::<()>(1);
    let (board_fetch_result_tx, mut board_fetch_result_rx) =
        mpsc::channel::<Vec<super::board::BoardStory>>(1);
    app.board_fetch_request_tx = Some(board_fetch_request_tx);
    tokio::spawn(async move {
        while board_fetch_request_rx.recv().await.is_some() {
            let fetch = super::board::fetch_board_stories();
            let bounded = tokio::time::timeout(std::time::Duration::from_secs(10), fetch).await;
            if let Ok(Ok(stories)) = bounded {
                // try_send (not send): if a previous result is still
                // sitting in the 1-slot channel, we'd rather drop this
                // one than block the listener task.
                let _ = board_fetch_result_tx.try_send(stories);
            }
        }
    });

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
                if app.lifecycle.should_quit {
                    super::ui_state::UiState::save(app, session);
                    app.save_usage_to_disk();
                    return Ok(());
                }
                // Check for P2P commands after handling input
                if let Some(cmd) = app.lifecycle.p2p_command.take() {
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
                        // Board-invalidate marker (#313) — every MCP
                        // mutation handler broadcasts this so connected
                        // TUIs refetch within ~2s instead of waiting up
                        // to 60s for the next poll tick. System-typed
                        // so it doesn't render as chat; consumed here
                        // and short-circuits before `handle_broadcast_message`.
                        if msg.message_type == crate::session::MessageType::System
                            && msg.text == BOARD_INVALIDATE_MARKER
                        {
                            if let Some(tx) = &board_poller_tx {
                                let _ = tx.try_send(
                                    super::board_poller::PollerCommand::RefreshNow,
                                );
                            }
                            if let Some(tx) = &app.board_fetch_request_tx {
                                let _ = tx.try_send(());
                            }
                            continue;
                        }
                        handle_broadcast_message(msg, app);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        match session.read_messages() {
                            Ok(messages) => {
                                // Rebuild classification cache after lag recovery
                                app.msg.class_cache = messages.iter().map(|m| {
                                    if m.is_chat() {
                                        Some(crate::session::classify(m, &app.identity.name))
                                    } else {
                                        None
                                    }
                                }).collect();
                                app.msg.messages = messages;
                                app.push_message(session::Message {
                                    from: "bpmux".to_string(),
                                    text: format!("Caught up — {n} messages arrived while processing."),
                                    ts: chrono::Utc::now(),
                                    message_type: crate::session::MessageType::System,
                                    ..session::Message::default()
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
                app.lifecycle.should_reload = true;
                return Ok(());
            }
            Some(dora) = dora_rx.recv() => {
                app.project.dora = Some(dora);
            }
            Some(state) = project_rx.recv() => {
                app.project.state = Some(state);
            }
            Some(stories) = board_fetch_result_rx.recv() => {
                // On-demand gh-CLI fetch landed (#294). Mirror into
                // both board.stories (read by handlers + render) and
                // board_snapshot (read by velocity zone's is_feature
                // lookup per #219 rework defect 3). Stamp freshness on
                // the snapshot so the panel footer shows a real
                // timestamp, matching the shape the old sync path had.
                if let Ok(mut snap) = app.board_snapshot.lock() {
                    snap.stories = stories.clone();
                    if snap.last_updated.is_none() {
                        snap.last_updated = Some(chrono::Utc::now());
                    }
                }
                app.board.stories = stories;
            }
            Some(injected) = injected_rx.recv() => {
                match injected {
                    TerminalInjectEvent::Clear => app.clear_input(),
                    TerminalInjectEvent::Submit => { let _ = app.submit_message(); }
                    other => match other.into_crossterm() {
                        Ok(event) => handle_terminal_event(event, app, session)?,
                        Err(e) => crate::session::diag::log_diag(&format!(
                            "terminal: dropping inject event: {}",
                            e.message()
                        )),
                    },
                }
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

/// Marker the bridge emits when an agent calls view_tokens.
/// Kept in sync with VIEW_TOKENS_MARKER in src/bridge/tools.rs.
pub(super) const VIEW_TOKENS_MARKER: &str = "__BPMUX_VIEW_TOKENS__";

/// Marker the bridge emits when an agent calls view_roadmap.
/// Kept in sync with VIEW_ROADMAP_MARKER in src/bridge/tools.rs.
pub(super) const VIEW_ROADMAP_MARKER: &str = "__BPMUX_VIEW_ROADMAP__";

/// Marker the bridge emits when an agent calls view_velocity (#210).
/// Kept in sync with VIEW_VELOCITY_MARKER in bridge/tools/view_velocity.rs.
pub(super) const VIEW_VELOCITY_MARKER: &str = "__BPMUX_VIEW_VELOCITY__";

/// Marker the bridge emits when an agent calls view_backlog (#286).
/// Kept in sync with VIEW_BACKLOG_MARKER in bridge/tools/view_backlog.rs.
pub(super) const VIEW_BACKLOG_MARKER: &str = "__BPMUX_VIEW_BACKLOG__";

/// Marker the bridge emits after every story-mutating MCP call so
/// every connected TUI triggers an immediate board refetch (#313).
/// Kept in sync with `BOARD_INVALIDATE_MARKER` in
/// `bridge/tools_stories.rs`.
pub(super) const BOARD_INVALIDATE_MARKER: &str = "__BPMUX_BOARD_INVALIDATE__";

fn handle_broadcast_message(msg: session::Message, app: &mut App) {
    // Agent-triggered view_tokens opens the overlay. Same
    // delegation pattern — System-type message + marker text; no
    // chat rendering, just the side effect.
    if msg.message_type == crate::session::MessageType::System && msg.text == VIEW_TOKENS_MARKER {
        app.handle_tokens();
        return;
    }
    // Agent-triggered view_roadmap runs the same handle_roadmap
    // path /roadmap uses — posts the roadmap text as a system message.
    if msg.message_type == crate::session::MessageType::System && msg.text == VIEW_ROADMAP_MARKER {
        app.handle_roadmap();
        return;
    }
    // Agent-triggered view_velocity opens the project panel where
    // velocity already lives (#227). No fetch — the board_poller
    // snapshot and in-memory message log supply the data the zone
    // renders from.
    if msg.message_type == crate::session::MessageType::System && msg.text == VIEW_VELOCITY_MARKER {
        app.handle_velocity();
        return;
    }
    // Agent-triggered view_backlog (#286) opens the project panel
    // scrolled/focused to the board section. Same non-clearing pattern
    // as view_velocity — the agent emitted this in response to a
    // prose-mention; Ryan's in-progress input must not be wiped.
    if msg.message_type == crate::session::MessageType::System && msg.text == VIEW_BACKLOG_MARKER {
        app.handle_view_backlog();
        return;
    }
    let self_identity = app.from_identity();
    if msg.from != self_identity {
        let short_name = msg.from.split(':').next().unwrap_or(&msg.from).to_string();
        let preview: String = msg.text.chars().take(NOTIFICATION_PREVIEW_CHARS).collect();
        let is_dm = msg.to.as_ref().is_some_and(|r| {
            r.iter().any(|n| n == &app.identity.name)
                && !r
                    .iter()
                    .all(|n| n == crate::session::message::BROADCAST_ALL)
        });
        app.receive_message(msg);
        if !app.display.tui_focused {
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
