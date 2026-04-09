use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::time::Instant;

use crate::session::Session;
use crate::session::bayes::BayesModel;
use crate::session::classifier;
use crate::session::message::{Message, MessageType};

const PAUSE_THRESHOLD: Duration = Duration::from_secs(30);

/// Spawn a task that monitors the session for corrections and nudges
/// agents to reflect after a natural pause.
///
/// The nudge is a chat message from "bpmux" directed to all connected
/// agents (not broadcast to the owner's TUI). Agents see it as a
/// channel notification and can choose to call save_lesson.
pub fn spawn_nudge_task(session: Arc<Session>, owner_name: String) {
    let mut rx = session.subscribe();
    let messages = session.read_messages().unwrap_or_default();

    tokio::spawn(async move {
        let mut pending_corrections: Vec<String> = Vec::new();
        let mut last_message_at = Instant::now();
        let mut known_agents: Vec<String> = Vec::new();
        let mut model = BayesModel::default();
        if !messages.is_empty() {
            model.train(&messages, &owner_name);
        }

        loop {
            let timeout = if pending_corrections.is_empty() {
                Duration::from_secs(3600)
            } else {
                let elapsed = last_message_at.elapsed();
                PAUSE_THRESHOLD.saturating_sub(elapsed)
            };

            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            last_message_at = Instant::now();
                            track_agents(&msg, &owner_name, &mut known_agents);
                            let class = if model.is_trained() {
                                model.classify(&msg, &owner_name)
                            } else {
                                classifier::classify(&msg, &owner_name)
                            };
                            if class.is_experience() {
                                let preview: String = msg.text.chars().take(80).collect();
                                pending_corrections.push(preview);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = tokio::time::sleep(timeout), if !pending_corrections.is_empty() => {
                    if !known_agents.is_empty() {
                        send_nudge(&session, &pending_corrections, &known_agents);
                    }
                    pending_corrections.clear();
                }
            }
        }
    });
}

pub(crate) fn track_agents(msg: &Message, owner_name: &str, agents: &mut Vec<String>) {
    if msg.message_type == MessageType::System {
        if let Some(peer) = msg.text.strip_suffix(" joined") {
            let short = peer.split(':').next().unwrap_or(peer);
            if short != owner_name && !agents.contains(&short.to_string()) {
                agents.push(short.to_string());
            }
        } else if let Some(peer) = msg.text.strip_suffix(" left") {
            let short = peer.split(':').next().unwrap_or(peer);
            agents.retain(|a| a != short);
        }
    }
}

fn send_nudge(session: &Session, corrections: &[String], agents: &[String]) {
    let nudge_text = build_nudge_text(corrections);
    let nudge = Message {
        from: "bpmux".to_string(),
        text: nudge_text,
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        to: Some(agents.to_vec()),
    };
    let _ = session.broadcast(&nudge);
}

pub(crate) fn build_nudge_text(experiences: &[String]) -> String {
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
