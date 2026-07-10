use std::time::Duration;
use tokio::time::Instant;

use crate::hub::Hub;
use message::{Message, MessageType};

use super::agents::AgentTracker;

const HANDOFF_TIMEOUT: Duration = Duration::from_secs(60);

const HANDOFF_ACTIONS: &[&str] = &[
    "desk check",
    "review",
    "accept",
    "pick up",
    "your turn",
    "delivered",
    "ready for",
    "needs your",
];

#[derive(Default)]
pub struct HandoffTracker {
    pending: Option<PendingHandoff>,
}

struct PendingHandoff {
    from: String,
    to: String,
    when: Instant,
    ts: chrono::DateTime<chrono::Utc>,
}

impl HandoffTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_message(
        &mut self,
        msg: &Message,
        sender: &str,
        agents: &AgentTracker,
        owner_name: &str,
    ) {
        if !msg.is_chat() || sender == "bpmux" {
            return;
        }

        if let Some(recipient) = detect_handoff(&msg.text, &agents.connected, owner_name) {
            self.pending = Some(PendingHandoff {
                from: sender.to_string(),
                to: recipient,
                when: Instant::now(),
                ts: msg.ts,
            });
        }

        if let Some(ref h) = self.pending
            && sender == h.to
        {
            self.pending = None;
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.pending.as_ref().map(|h| h.when + HANDOFF_TIMEOUT)
    }

    pub fn timeout_nudge(&mut self, hub: &Hub) {
        let Some(h) = self.pending.take() else {
            return;
        };
        const RESPONSE_SCAN_WINDOW: usize = 200;
        if let Ok(messages) = hub.read_recent(RESPONSE_SCAN_WINDOW, 0, &|_| true) {
            let recipient_responded = messages.iter().any(|m| {
                if !m.is_chat() {
                    return false;
                }
                let sender = m.from.split(':').next().unwrap_or(&m.from);
                sender == h.to && m.ts > h.ts
            });
            if recipient_responded {
                return;
            }
        }

        let nudge = Message {
            from: "bpmux".to_string(),
            text: format!(
                "{} hasn't acknowledged the handoff from {}. Waiting for response.",
                h.to, h.from
            ),
            ts: chrono::Utc::now(),
            message_type: MessageType::Nudge,
            to: Some(vec![h.to]),
            ..Message::default()
        };
        let _ = hub.broadcast(&nudge);
    }
}

fn detect_handoff(text: &str, agents: &[String], owner_name: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let mut candidates: Vec<&str> = agents.iter().map(|s| s.as_str()).collect();
    candidates.push(owner_name);

    for name in &candidates {
        let patterns = [format!("{name},"), format!("{name} —"), format!("{name}:")];
        if patterns
            .iter()
            .any(|p| lower.starts_with(&p.to_lowercase()))
            && HANDOFF_ACTIONS.iter().any(|a| lower.contains(a))
        {
            return Some(name.to_string());
        }
    }
    None
}
