//! Detects teaching moments and nudges agents to save lessons after pauses.

use std::time::Duration;
use tokio::time::Instant;

use crate::session::Session;
use crate::session::bayes::BayesModel;
use crate::session::classifier;
use crate::session::message::{Message, MessageType};

use super::agents::AgentTracker;

const PAUSE_THRESHOLD: Duration = Duration::from_secs(30);

pub struct ExperienceDetector {
    pending: Vec<String>,
    last_message_at: Instant,
    model: BayesModel,
    owner_name: String,
}

impl ExperienceDetector {
    pub fn new(messages: &[Message], owner_name: &str) -> Self {
        let mut model = BayesModel::default();
        if !messages.is_empty() {
            model.train(messages, owner_name);
        }
        Self {
            pending: Vec::new(),
            last_message_at: Instant::now(),
            model,
            owner_name: owner_name.to_string(),
        }
    }

    pub fn on_message(&mut self, msg: &Message, _sender: &str) {
        self.last_message_at = Instant::now();
        let class = if self.model.is_trained() {
            self.model.classify(msg, &self.owner_name)
        } else {
            classifier::classify(msg, &self.owner_name)
        };
        if class.is_experience() {
            let preview: String = msg.text.chars().take(80).collect();
            self.pending.push(preview);
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn timeout(&self) -> Duration {
        if self.pending.is_empty() {
            Duration::from_secs(3600)
        } else {
            PAUSE_THRESHOLD.saturating_sub(self.last_message_at.elapsed())
        }
    }

    pub fn flush(&mut self, session: &Session, agents: &AgentTracker) {
        if !agents.connected.is_empty() {
            let nudge_text = build_nudge_text(&self.pending);
            let nudge = Message {
                from: "bpmux".to_string(),
                text: nudge_text,
                ts: chrono::Utc::now(),
                message_type: MessageType::Chat,
                to: Some(agents.connected.clone()),
                addressee: None,
                explicit_privacy: false,
            };
            let _ = session.broadcast(&nudge);
        }
        self.pending.clear();
    }
}

fn build_nudge_text(experiences: &[String]) -> String {
    let count = experiences.len();
    let mut text = format!(
        "{count} experience{} noted. Consider saving what you learned:\n",
        if count == 1 { "" } else { "s" }
    );
    for (i, preview) in experiences.iter().enumerate() {
        text.push_str(&format!("  {}. {preview}\n", i + 1));
    }
    text.push_str("Use save_lesson(topic, lesson) to capture your understanding.");
    text
}
