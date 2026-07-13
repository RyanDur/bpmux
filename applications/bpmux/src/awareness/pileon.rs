use std::time::Duration;
use tokio::time::Instant;

use crate::awareness::AwarenessHost;
use comprehension::classifier;
use message::Message;

use super::agents::AgentTracker;

const PILEON_WINDOW: Duration = Duration::from_secs(5);
const PILEON_THRESHOLD: usize = 3;

pub struct PileonDetector {
    window_start: Option<Instant>,
    responders: Vec<String>,
    owner_name: String,
}

impl PileonDetector {
    pub fn new(owner_name: &str) -> Self {
        Self {
            window_start: None,
            responders: Vec::new(),
            owner_name: owner_name.to_string(),
        }
    }

    pub fn on_message(
        &mut self,
        host: &dyn AwarenessHost,
        msg: &Message,
        sender: &str,
        _agents: &AgentTracker,
    ) {
        if !msg.is_chat() {
            return;
        }

        if sender == self.owner_name && msg.to.is_none() {
            self.window_start = Some(Instant::now());
            self.responders.clear();
            return;
        }

        if sender != self.owner_name
            && sender != "bpmux"
            && self
                .window_start
                .is_some_and(|t| t.elapsed() < PILEON_WINDOW)
            && !is_coordination(&msg.text)
            && !self.responders.contains(&sender.to_string())
        {
            self.responders.push(sender.to_string());
            if self.responders.len() >= PILEON_THRESHOLD {
                let nudge = Message {
                    from: "bpmux".to_string(),
                    text: format!(
                        "{} responses in quick succession. Was one voice enough?",
                        self.responders.len()
                    ),
                    ts: chrono::Utc::now(),
                    message_type: message::MessageType::Nudge,
                    to: Some(self.responders.clone()),
                    ..Message::default()
                };
                let _ = host.broadcast(&nudge);
                self.window_start = None;
                self.responders.clear();
            }
        }
    }
}

fn is_coordination(text: &str) -> bool {
    let lower = text.to_lowercase();
    classifier::COORDINATION_PATTERNS
        .iter()
        .any(|p| lower.contains(p))
}
