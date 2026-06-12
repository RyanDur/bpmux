use std::sync::Arc;

use tokio::sync::broadcast;

use crate::room::LivePtyRegistry;
use crate::session::Session;
use crate::session::message::{BROADCAST_ALL, Message};
use crate::session::routing_identity::RoutingStore;
use crate::session::terminal_inject::frame_team_message;
use crate::transport::ws::short_name;

const SUBMIT: char = '\r';

pub fn spawn_terminal_delivery_loop(
    session: Arc<Session>,
    registry: Arc<LivePtyRegistry>,
    routing: Arc<RoutingStore>,
) {
    let mut rx = session.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => deliver_to_terminals(&msg, &registry, &routing),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

pub fn deliver_to_terminals(msg: &Message, registry: &LivePtyRegistry, routing: &RoutingStore) {
    if !msg.is_chat() {
        return;
    }
    let payload = framed_payload(msg);
    let ts = msg.ts.to_rfc3339();
    for (session_id, terminal_id) in routing.delivery_targets(msg.to.as_deref(), &msg.from) {
        if !after_cursor(routing, &session_id, msg.ts) {
            continue;
        }
        if registry.write_to(&terminal_id, payload.as_bytes()) {
            routing.mark_delivered(&session_id, &ts);
        }
    }
}

pub fn drain_missed(
    session: &Session,
    registry: &LivePtyRegistry,
    routing: &RoutingStore,
    session_id: &str,
    terminal_id: &str,
    display: &str,
) {
    let messages = match session.read_messages() {
        Ok(messages) => messages,
        Err(_) => return,
    };
    for msg in messages {
        if !msg.is_chat() || short_name(&msg.from) == display || !reaches(&msg, display) {
            continue;
        }
        if !after_cursor(routing, session_id, msg.ts) {
            continue;
        }
        if registry.write_to(terminal_id, framed_payload(&msg).as_bytes()) {
            routing.mark_delivered(session_id, &msg.ts.to_rfc3339());
        }
    }
}

fn framed_payload(msg: &Message) -> String {
    let scope = scope_label(msg.to.as_deref());
    let framed = frame_team_message(short_name(&msg.from), &scope, &msg.text);
    format!("{framed}{SUBMIT}")
}

fn after_cursor(
    routing: &RoutingStore,
    session_id: &str,
    ts: chrono::DateTime<chrono::Utc>,
) -> bool {
    match routing
        .cursor(session_id)
        .as_deref()
        .and_then(|cursor| chrono::DateTime::parse_from_rfc3339(cursor).ok())
    {
        Some(cursor) => ts > cursor.with_timezone(&chrono::Utc),
        None => true,
    }
}

fn reaches(msg: &Message, display: &str) -> bool {
    match msg.to.as_deref() {
        None => true,
        Some(names) if names.iter().any(|name| name == BROADCAST_ALL) => true,
        Some(names) => names.iter().any(|name| short_name(name) == display),
    }
}

fn scope_label(to: Option<&[String]>) -> String {
    match to {
        None => "broadcast".to_string(),
        Some(names) if names.iter().any(|name| name == BROADCAST_ALL) => "broadcast".to_string(),
        Some(names) if names.len() == 1 => "→you".to_string(),
        Some(names) => format!(
            "to:{}",
            names
                .iter()
                .map(|name| short_name(name))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::room::spawn_pty_retained;
    use crate::session::MessageType;

    fn chat(from: &str, text: &str, to: Option<Vec<String>>) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: chrono::Utc::now(),
            message_type: MessageType::Chat,
            to,
            ..Message::default()
        }
    }

    async fn drain_for(rx: &mut broadcast::Receiver<Vec<u8>>, needle: &str) -> String {
        let mut out = String::new();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(4);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(bytes)) if bytes.is_empty() => break,
                Ok(Ok(bytes)) => {
                    out.push_str(&String::from_utf8_lossy(&bytes));
                    if out.contains(needle) {
                        break;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        out
    }

    fn register_live_cat(
        registry: &LivePtyRegistry,
        routing: &RoutingStore,
        session_id: &str,
        display: &str,
        terminal_id: &str,
    ) -> broadcast::Receiver<Vec<u8>> {
        let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
        let rx = live.subscribe();
        registry.insert(terminal_id.to_string(), live);
        routing.register_agent(session_id, display, terminal_id, "2026-06-12T10:00:00Z");
        rx
    }

    #[tokio::test]
    async fn a_broadcast_chat_is_framed_and_written_into_a_live_agents_terminal() {
        let dir = tempfile::TempDir::new().unwrap();
        let routing = RoutingStore::load(dir.path().to_path_buf());
        let registry = LivePtyRegistry::new();
        let mut rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

        deliver_to_terminals(&chat("pm", "ship it", None), &registry, &routing);

        let seen = drain_for(&mut rx, "ship it").await;
        assert!(
            seen.contains("⟦bpmux⟧ pm broadcast▸") && seen.contains("ship it"),
            "a broadcast chat is delivered INTO the agent's terminal as a forgery-framed line (the agnostic receive — no agent-specific glue); got {seen:?}"
        );
    }

    #[tokio::test]
    async fn the_carriage_return_submits_the_line_so_a_bare_read_loop_processes_it_not_parks() {
        let dir = tempfile::TempDir::new().unwrap();
        let routing = RoutingStore::load(dir.path().to_path_buf());
        let registry = LivePtyRegistry::new();
        let live = spawn_pty_retained(
            "/bin/sh",
            &[
                "-c".into(),
                "while read line; do printf 'PROCESSED[%s]\\n' \"$line\"; done".into(),
            ],
            None,
            &[],
        )
        .unwrap();
        let mut rx = live.subscribe();
        registry.insert("term-dev".to_string(), live);
        routing.register_agent("sess-dev", "dev", "term-dev", "2000-01-01T00:00:00Z");

        deliver_to_terminals(&chat("pm", "reach-the-role", None), &registry, &routing);

        let seen = drain_for(&mut rx, "PROCESSED[").await;
        assert!(
            seen.contains("PROCESSED[") && seen.contains("reach-the-role"),
            "the \\r-submit makes a bare read-loop RETURN from `read` and PROCESS the line — it reaches the role processed, NOT parked awaiting a manual ENTER (falsifier-4, the 2026-06-06 reject, at the AGNOSTIC level: a non-Claude script, zero glue). Without the submit, `read` would block forever and PROCESSED[ would never appear; got {seen:?}"
        );
    }

    #[tokio::test]
    async fn a_scoped_message_reaches_only_the_matching_display_no_leak() {
        let dir = tempfile::TempDir::new().unwrap();
        let routing = RoutingStore::load(dir.path().to_path_buf());
        let registry = LivePtyRegistry::new();
        let mut dev_rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");
        let mut pm_rx = register_live_cat(&registry, &routing, "sess-pm", "pm", "term-pm");

        deliver_to_terminals(
            &chat("designer", "for dev only", Some(vec!["dev".to_string()])),
            &registry,
            &routing,
        );

        let dev_seen = drain_for(&mut dev_rx, "for dev only").await;
        assert!(
            dev_seen.contains("⟦bpmux⟧ designer →you▸") && dev_seen.contains("for dev only"),
            "a to:[dev] message resolves dev's DISPLAY to its id and reaches dev's terminal, scoped →you; got {dev_seen:?}"
        );

        registry.write_to("term-pm", b"PM-SENTINEL\r");
        let pm_seen = drain_for(&mut pm_rx, "PM-SENTINEL").await;
        assert!(
            !pm_seen.contains("for dev only"),
            "no-leak: a message scoped to a DIFFERENT display must NOT reach pm's terminal (the sentinel barrier proves nothing arrived before it); got {pm_seen:?}"
        );
    }

    #[tokio::test]
    async fn the_sender_does_not_receive_an_echo_of_its_own_message() {
        let dir = tempfile::TempDir::new().unwrap();
        let routing = RoutingStore::load(dir.path().to_path_buf());
        let registry = LivePtyRegistry::new();
        let mut dev_rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

        deliver_to_terminals(&chat("dev", "my own line", None), &registry, &routing);

        registry.write_to("term-dev", b"OWN-SENTINEL\r");
        let seen = drain_for(&mut dev_rx, "OWN-SENTINEL").await;
        assert!(
            !seen.contains("my own line"),
            "own-sender suppression: a broadcast must not be injected back into the SENDER's own terminal (no self-echo loop); got {seen:?}"
        );
    }

    #[tokio::test]
    async fn offline_then_reconnect_drains_the_missed_message_not_the_already_delivered_one() {
        let store_dir = tempfile::TempDir::new().unwrap();
        let routing = RoutingStore::load(store_dir.path().to_path_buf());
        let registry = LivePtyRegistry::new();
        let sess_dir = tempfile::TempDir::new().unwrap();
        let (tx, _rx) = broadcast::channel(16);
        let session =
            crate::session::Session::new(sess_dir.path().to_path_buf(), "ryan".to_string(), tx);

        let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
        let mut rx1 = live.subscribe();
        registry.insert("term1".to_string(), live);
        routing.register_agent("sess-dev", "dev", "term1", "2000-01-01T00:00:00Z");

        let m1 = chat("pm", "M1-delivered-live", None);
        session.broadcast(&m1).unwrap();
        deliver_to_terminals(&m1, &registry, &routing);
        assert!(
            drain_for(&mut rx1, "M1-delivered-live")
                .await
                .contains("M1-delivered-live"),
            "M1 reaches dev live while online"
        );

        registry.remove("term1");
        std::thread::sleep(std::time::Duration::from_millis(5));

        let m2 = chat("designer", "M2-while-offline", None);
        session.broadcast(&m2).unwrap();
        deliver_to_terminals(&m2, &registry, &routing);

        let live2 = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
        let mut rx2 = live2.subscribe();
        registry.insert("term2".to_string(), live2);
        routing.register_agent("sess-dev", "dev", "term2", "2026-06-12T20:00:00Z");
        drain_missed(&session, &registry, &routing, "sess-dev", "term2", "dev");

        registry.write_to("term2", b"DRAIN-SENTINEL\r");
        let drained = drain_for(&mut rx2, "DRAIN-SENTINEL").await;
        assert!(
            drained.contains("M2-while-offline"),
            "a reconnecting agent drains the message it missed while OFFLINE (falsifier-3 no-loss — reach = live OR on-reconnect, never lost); got {drained:?}"
        );
        assert!(
            !drained.contains("M1-delivered-live"),
            "the already-live-delivered M1 is NOT re-delivered — the cursor advanced on live-delivery so reconnect drains only the gap (no double); got {drained:?}"
        );
    }

    #[tokio::test]
    async fn a_pty_loop_delivered_message_is_not_systematically_re_drained_by_the_dev_stop_hook() {
        let dir = tempfile::TempDir::new().unwrap();
        let routing = RoutingStore::load(dir.path().to_path_buf());
        let registry = LivePtyRegistry::new();
        let live = spawn_pty_retained("/bin/cat", &[], None, &[]).unwrap();
        let _rx = live.subscribe();
        registry.insert("term-dev".to_string(), live);
        routing.register_agent("sess-dev", "dev", "term-dev", "2000-01-01T00:00:00Z");

        let m = chat("pm", "single-delivery", None);
        deliver_to_terminals(&m, &registry, &routing);

        let floor = routing
            .drain_floor("sess-dev", Some("2020-01-01T00:00:00Z"))
            .expect("floor");
        let floor_ts = chrono::DateTime::parse_from_rfc3339(&floor)
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert!(
            floor_ts >= m.ts,
            "after the PTY-loop delivers M and advances the shared #1712 id-cursor, the still-firing DEV Stop-hook's wait-for-event drain (drain_floor) clamps a lagging client cursor FORWARD to >= M — so first_pending_after(floor) excludes M and the Stop-hook does NOT re-deliver it: SYSTEMATIC no-double via the shared cursor (whichever delivers first advances it). The narrow concurrent race (both read the cursor before either advances) is bounded-as-DEV-transitional — prod post-#1658 runs ONLY the PTY-loop. floor {floor:?} vs M.ts {}",
            m.ts
        );
    }

    #[tokio::test]
    async fn a_non_chat_message_is_never_injected_into_a_terminal() {
        let dir = tempfile::TempDir::new().unwrap();
        let routing = RoutingStore::load(dir.path().to_path_buf());
        let registry = LivePtyRegistry::new();
        let mut rx = register_live_cat(&registry, &routing, "sess-dev", "dev", "term-dev");

        let mut state = chat("bpmux", "STATE-BODY", None);
        state.message_type = MessageType::StateSnapshot;
        deliver_to_terminals(&state, &registry, &routing);

        registry.write_to("term-dev", b"NONCHAT-SENTINEL\r");
        let seen = drain_for(&mut rx, "NONCHAT-SENTINEL").await;
        assert!(
            !seen.contains("STATE-BODY"),
            "only chat/insight messages are injected — a state-snapshot must never be written into a PTY; got {seen:?}"
        );
    }
}
