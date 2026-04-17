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

/// A classified experience awaiting nudge emission. Stores the sender's
/// short role name alongside the preview so that `flush` can filter each
/// recipient's nudge to exclude their own messages (#163 tuning 1).
#[derive(Debug, Clone)]
struct PendingExperience {
    sender: String,
    preview: String,
}

pub struct ExperienceDetector {
    pending: Vec<PendingExperience>,
    last_message_at: Instant,
    /// Last time we emitted a `CLASSIFY` diag line. Used to rate-limit
    /// classifier observability logging at 1/sec. See `#159`.
    last_classify_log: Option<Instant>,
    /// Snapshot of `BPMUX_CLASSIFY_LOG` at construction time. Cached here
    /// (rather than re-reading the env var on every message) because env
    /// access is process-global, expensive, and — in modern Rust — marked
    /// `unsafe` to mutate. Production reads it once at startup; tests can
    /// set it directly via `with_classify_log`.
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

    /// Construct with explicit classify-log flag. Production uses
    /// `new()` which reads the env var. Tests pass the flag directly.
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
        // Per-recipient filtering (#163 tuning 1): each connected agent
        // gets a personalized nudge that excludes their own messages.
        // Eliminates save-own-message recursion at the nudge layer. An
        // empty filtered list for a recipient means they have nothing to
        // be nudged about (all pending previews came from themselves) —
        // skip that recipient entirely.
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
                // Structured type instead of the 📝 text prefix
                // as the filterable marker. The prefix stays in the
                // text for human readability but the filter predicate
                // is now type-based.
                message_type: MessageType::Nudge,
                to: Some(vec![recipient.clone()]),
                ..Message::default()
            };
            let _ = session.broadcast(&nudge);
        }
        self.pending.clear();
    }
}

/// Build the reflection nudge (#163 tuning 2). Reworded from the
/// earlier "N experiences noted. Consider saving what you learned"
/// phrasing to:
///   1. Prevent recursive self-classification by avoiding phrases that
///      matched the classifier's patterns. Combined with the Bayes
///      `sender == "bpmux"` guard (#163 tuning 2.5), this is defense in
///      depth against the nudge-classifies-itself loop.
///   2. Frame the previews as POINTERS to moments, not content to copy.
///      Makes the individuality principle explicit — each agent reflects
///      through their own lens, not by verbatim-copying others' words.
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
