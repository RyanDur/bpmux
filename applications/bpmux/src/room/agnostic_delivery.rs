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
    let scope = scope_label(msg.to.as_deref());
    let framed = frame_team_message(short_name(&msg.from), &scope, &msg.text);
    let payload = format!("{framed}{SUBMIT}");
    for terminal_id in routing.delivery_terminals(msg.to.as_deref(), &msg.from) {
        registry.write_to(&terminal_id, payload.as_bytes());
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
