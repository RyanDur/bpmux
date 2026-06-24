use std::time::Duration;
use tokio::time::Instant;

use crate::comprehension::bayes::BayesModel;
use crate::comprehension::classifier;
use crate::diag::log_diag;
use crate::session::Session;
use crate::session::message::{Message, MessageType};

use super::agents::AgentTracker;

const PAUSE_THRESHOLD: Duration = Duration::from_secs(30);
const CLASSIFY_LOG_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
struct PendingExperience {
    sender: String,
    preview: String,
}

pub struct ExperienceDetector {
    pending: Vec<PendingExperience>,
    last_message_at: Instant,
    last_classify_log: Option<Instant>,
    classify_log_enabled: bool,
    model: BayesModel,
    owner_name: String,
}

impl ExperienceDetector {
    pub fn new(messages: &[Message], owner_name: &str) -> Self {
        Self::with_classify_log(
            messages,
            owner_name,
            std::env::var("BPMUX_CLASSIFY_LOG").is_ok(),
        )
    }

    pub fn with_classify_log(
        messages: &[Message],
        owner_name: &str,
        classify_log_enabled: bool,
    ) -> Self {
        let mut model = BayesModel::default();
        if !messages.is_empty() {
            model.train(messages, owner_name);
        }
        Self {
            pending: Vec::new(),
            last_message_at: Instant::now(),
            last_classify_log: None,
            classify_log_enabled,
            model,
            owner_name: owner_name.to_string(),
        }
    }

    pub fn has_classify_log(&self) -> bool {
        self.last_classify_log.is_some()
    }

    pub fn on_message(&mut self, msg: &Message, sender: &str) {
        self.last_message_at = Instant::now();
        let class = if self.model.is_trained() {
            self.model.classify(msg, &self.owner_name)
        } else {
            classifier::classify(msg, &self.owner_name)
        };

        if self.classify_log_enabled {
            let now = Instant::now();
            let should_log = self
                .last_classify_log
                .map(|t| now.duration_since(t) >= CLASSIFY_LOG_INTERVAL)
                .unwrap_or(true);
            if should_log {
                self.last_classify_log = Some(now);
                let preview: String = msg
                    .text
                    .chars()
                    .take(80)
                    .collect::<String>()
                    .replace('\n', " ");
                log_diag(&format!(
                    "CLASSIFY sender={sender} class={class:?} preview={preview:?}"
                ));
            }
        }

        if class.is_experience() {
            let preview: String = msg.text.chars().take(80).collect();
            self.pending.push(PendingExperience {
                sender: sender.to_string(),
                preview,
            });
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
        for recipient in &agents.connected {
            let filtered: Vec<&PendingExperience> = self
                .pending
                .iter()
                .filter(|p| p.sender != *recipient)
                .collect();
            if filtered.is_empty() {
                continue;
            }
            let nudge_text = build_nudge_text(&filtered);
            let nudge = Message {
                from: "bpmux".to_string(),
                text: nudge_text,
                ts: chrono::Utc::now(),
                message_type: MessageType::Nudge,
                to: Some(vec![recipient.clone()]),
                ..Message::default()
            };
            let _ = session.broadcast(&nudge);
        }
        self.pending.clear();
    }
}

fn build_nudge_text(experiences: &[&PendingExperience]) -> String {
    let count = experiences.len();
    let mut text = format!(
        "\u{1F4DD} {count} candidate moment{} from recent chat. Pointers, not content to copy — reflect through your own role's lens and save your own framing if any apply:\n",
        if count == 1 { "" } else { "s" }
    );
    for (i, entry) in experiences.iter().enumerate() {
        text.push_str(&format!(
            "  {}. [from {}] {}\n",
            i + 1,
            entry.sender,
            entry.preview
        ));
    }
    text.push_str("Call save_lesson(topic, lesson) only for durable rules in your own words.");
    text
}
