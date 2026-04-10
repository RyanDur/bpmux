//! Detects teaching moments and nudges agents to save lessons after pauses.

use std::time::Duration;
use tokio::time::Instant;

use crate::session::Session;
use crate::session::bayes::BayesModel;
use crate::session::classifier;
use crate::session::diag::log_diag;
use crate::session::message::{Message, MessageType};

use super::agents::AgentTracker;

const PAUSE_THRESHOLD: Duration = Duration::from_secs(30);
/// Rate limit for classifier decision logging (#159). Prevents flooding
/// the diag log on high-traffic sessions. One line per second per
/// session is enough to characterize the classifier's behavior without
/// drowning the log.
const CLASSIFY_LOG_INTERVAL: Duration = Duration::from_secs(1);

pub struct ExperienceDetector {
    pending: Vec<String>,
    last_message_at: Instant,
    /// Last time we emitted a `CLASSIFY` diag line. Used to rate-limit
    /// classifier observability logging at 1/sec. See `#159`.
    last_classify_log: Option<Instant>,
    /// Snapshot of `BPMUX_CLASSIFY_LOG` at construction time. Cached here
    /// (rather than re-reading the env var on every message) because env
    /// access is process-global, expensive, and — in modern Rust — marked
    /// `unsafe` to mutate. Production reads it once at startup; tests can
    /// set it directly via `set_classify_log_for_test`.
    classify_log_enabled: bool,
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
            last_classify_log: None,
            classify_log_enabled: std::env::var("BPMUX_CLASSIFY_LOG").is_ok(),
            model,
            owner_name: owner_name.to_string(),
        }
    }

    /// Test-only override for the classify-log flag. Production reads
    /// `BPMUX_CLASSIFY_LOG` at construction; tests set the flag directly
    /// to avoid mutating process-global env in a crate that denies
    /// unsafe code.
    #[cfg(test)]
    pub fn set_classify_log_for_test(&mut self, enabled: bool) {
        self.classify_log_enabled = enabled;
    }

    #[cfg(test)]
    pub fn classify_log_count(&self) -> usize {
        if self.last_classify_log.is_some() {
            1
        } else {
            0
        }
    }

    pub fn on_message(&mut self, msg: &Message, sender: &str) {
        self.last_message_at = Instant::now();
        let class = if self.model.is_trained() {
            self.model.classify(msg, &self.owner_name)
        } else {
            classifier::classify(msg, &self.owner_name)
        };

        // Classifier decision logging (#159). Gated on the
        // BPMUX_CLASSIFY_LOG env var (snapshotted at construction).
        // Rate-limited to 1/sec per session via `last_classify_log`.
        // Purely observational — never affects classifier behavior.
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
