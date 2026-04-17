//! Tracks explicit handoffs and nudges if unacknowledged.

use std::time::Duration;
use tokio::time::Instant;

use crate::session::Session;
use crate::session::message::{Message, MessageType};

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
    /// Wall-clock timestamp of the handoff message, used by the
    /// conversation-state check in `timeout_nudge` to query the session
    /// log for responses that arrived after this point. `when` (Instant)
    /// is for tokio deadline math; `ts` is for comparing against
    /// persisted `Message::ts` values.
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

    pub fn timeout_nudge(&mut self, session: &Session) {
        let Some(h) = self.pending.take() else {
            return;
        };
        // Conversation-state check. Before emitting the "X hasn't
        // acknowledged the handoff" nudge, query the session log for
        // any messages from `h.to` with a timestamp after the
        // handoff. If the recipient has spoken since, suppress the
        // nudge — the handoff WAS acknowledged, and the in-memory
        // clear may have been missed due to tokio::select! race
        // timing or message-routing delay.
        //
        // This is defense-in-depth against the observed misfire pattern:
        // the in-memory `sender == h.to` clear on line ~60 above already
        // handles the common case, but the misfire log documented three
        // instances where the nudge fired while the recipient HAD
        // responded. The session log is the durable ground truth.
        if let Ok(messages) = session.read_messages() {
            let recipient_responded = messages.iter().any(|m| {
                if !m.is_chat() {
                    return false;
                }
                let sender = m.from.split(':').next().unwrap_or(&m.from);
                sender == h.to && m.ts > h.ts
            });
            if recipient_responded {
                return; // suppressed — recipient already spoke after handoff
            }
        }

        let nudge = Message {
            from: "bpmux".to_string(),
            text: format!(
                "{} hasn't acknowledged the handoff from {}. Waiting for response.",
                h.to, h.from
            ),
            ts: chrono::Utc::now(),
            // Classifier-authored nudges carry `Nudge` instead
            // of `Chat` so the default view filter catches them by
            // type rather than by the fragile text-prefix check that
            // caught only 📝-marked messages.
            message_type: MessageType::Nudge,
            to: Some(vec![h.to]),
            addressee: None,
            explicit_privacy: false,
            thread_id: None,
            multi_depth: None,
        };
        let _ = session.broadcast(&nudge);
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
