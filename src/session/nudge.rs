use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::time::Instant;

use crate::session::Session;
use crate::session::bayes::BayesModel;
use crate::session::classifier;
use crate::session::lessons;
use crate::session::message::{Message, MessageType};

const PAUSE_THRESHOLD: Duration = Duration::from_secs(30);
const MAX_RECALL_NUDGES: usize = 3;
const PILEON_WINDOW: Duration = Duration::from_secs(5);
const PILEON_THRESHOLD: usize = 3;

/// Spawn a task that monitors the session for experiences and nudges
/// agents to reflect after a natural pause. Also surfaces relevant
/// lessons when conversation touches known topics.
pub fn spawn_nudge_task(session: Arc<Session>, owner_name: String) {
    let mut rx = session.subscribe();
    let messages = session.read_messages().unwrap_or_default();

    tokio::spawn(async move {
        let mut pending_experiences: Vec<String> = Vec::new();
        let mut last_message_at = Instant::now();
        let mut known_agents: Vec<String> = Vec::new();
        let mut model = BayesModel::default();
        if !messages.is_empty() {
            model.train(&messages, &owner_name);
        }

        // Context-triggered recall state
        let mut surfaced_topics: HashSet<String> = HashSet::new();
        let mut recall_count: usize = 0;
        let mut cached_index = lessons::load_index(&session.dir);

        // Pile-on detection state
        let mut pileon_window_start: Option<Instant> = None;
        let mut pileon_responders: Vec<String> = Vec::new();

        loop {
            let timeout = if pending_experiences.is_empty() {
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

                            let sender = msg.from.split(':').next().unwrap_or(&msg.from);

                            // Experience detection
                            let class = if model.is_trained() {
                                model.classify(&msg, &owner_name)
                            } else {
                                classifier::classify(&msg, &owner_name)
                            };
                            if class.is_experience() {
                                let preview: String = msg.text.chars().take(80).collect();
                                pending_experiences.push(preview);
                            }

                            // Refresh cached index when new lessons are saved
                            if sender == "bpmux" && msg.text.contains("lesson saved") {
                                cached_index = lessons::load_index(&session.dir);
                            }

                            // Context-triggered recall: check agent messages against lesson topics
                            if msg.is_chat()
                                && sender != owner_name
                                && sender != "bpmux"
                                && recall_count < MAX_RECALL_NUDGES
                            {
                                check_recall_cached(
                                    &session,
                                    &msg,
                                    sender,
                                    &cached_index,
                                    &mut surfaced_topics,
                                    &mut recall_count,
                                );
                            }

                            // Pile-on detection
                            if msg.is_chat() && sender == owner_name && msg.to.is_none() {
                                pileon_window_start = Some(Instant::now());
                                pileon_responders.clear();
                            } else if msg.is_chat()
                                && sender != owner_name
                                && sender != "bpmux"
                                && pileon_window_start.is_some_and(|t| t.elapsed() < PILEON_WINDOW)
                                && !is_coordination(&msg.text)
                                && !pileon_responders.contains(&sender.to_string())
                            {
                                pileon_responders.push(sender.to_string());
                                if pileon_responders.len() >= PILEON_THRESHOLD {
                                    send_pileon_nudge(&session, &pileon_responders);
                                    pileon_window_start = None;
                                    pileon_responders.clear();
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = tokio::time::sleep(timeout), if !pending_experiences.is_empty() => {
                    if !known_agents.is_empty() {
                        send_experience_nudge(&session, &pending_experiences, &known_agents);
                    }
                    pending_experiences.clear();
                }
            }
        }
    });
}

fn check_recall_cached(
    session: &Session,
    msg: &Message,
    sender: &str,
    index: &lessons::LessonIndex,
    surfaced_topics: &mut HashSet<String>,
    recall_count: &mut usize,
) {
    if index.lessons.is_empty() {
        return;
    }

    let msg_keywords = lessons::extract_keywords(&msg.text);
    if msg_keywords.is_empty() {
        return;
    }

    // Find matching lessons, prioritize stalest (least recently reinforced)
    let mut matches: Vec<&lessons::AgentLesson> = index
        .lessons
        .iter()
        .filter(|l| {
            !surfaced_topics.contains(&l.topic)
                && lessons::extract_keywords(&l.topic)
                    .iter()
                    .any(|tk| msg_keywords.iter().any(|mk| mk == tk))
        })
        .collect();

    matches.sort_by(|a, b| {
        let a_time = a.last_reinforced.as_deref().unwrap_or(&a.established);
        let b_time = b.last_reinforced.as_deref().unwrap_or(&b.established);
        a_time.cmp(b_time) // oldest first = stalest first
    });

    if let Some(lesson) = matches.first() {
        let trend = reinforcement_trend(lesson);
        let nudge = Message {
            from: "bpmux".to_string(),
            text: format!(
                "Watch topic \"{}\" ({trend}). Team learned: {}\nUse search_log(\"{}\") for more context.",
                lesson.topic, lesson.lesson, lesson.topic
            ),
            ts: chrono::Utc::now(),
            message_type: MessageType::Chat,
            to: Some(vec![sender.to_string()]),
        };
        let _ = session.broadcast(&nudge);
        surfaced_topics.insert(lesson.topic.clone());
        *recall_count += 1;
    }
}

fn reinforcement_trend(lesson: &lessons::AgentLesson) -> String {
    let count = lesson.reinforcement_count;
    if count == 0 {
        return "new".to_string();
    }
    let last = lesson
        .last_reinforced
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok());
    match last {
        Some(ts) => {
            let age = chrono::Utc::now() - ts;
            if age.num_hours() < 24 {
                format!("reinforced {count}x, active")
            } else if age.num_days() < 3 {
                format!("reinforced {count}x, recent")
            } else {
                format!("reinforced {count}x, fading")
            }
        }
        None => format!("reinforced {count}x"),
    }
}

fn is_coordination(text: &str) -> bool {
    let lower = text.to_lowercase();
    crate::session::classifier::COORDINATION_PATTERNS
        .iter()
        .any(|p| lower.contains(p))
}

fn send_pileon_nudge(session: &Session, responders: &[String]) {
    let nudge = Message {
        from: "bpmux".to_string(),
        text: format!(
            "{} responses in quick succession. Was one voice enough?",
            responders.len()
        ),
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        to: Some(responders.to_vec()),
    };
    let _ = session.broadcast(&nudge);
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

fn send_experience_nudge(session: &Session, experiences: &[String], agents: &[String]) {
    let nudge_text = build_nudge_text(experiences);
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
