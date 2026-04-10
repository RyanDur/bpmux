//! Surfaces relevant lessons when conversation touches known topics.

use std::collections::HashSet;
use std::path::Path;

use crate::session::Session;
use crate::session::lessons;
use crate::session::message::{Message, MessageType};

const MAX_RECALL_NUDGES: usize = 3;

pub struct RecallSystem {
    surfaced_topics: HashSet<String>,
    recall_count: usize,
    cached_index: lessons::LessonIndex,
}

impl RecallSystem {
    pub fn new(session_dir: &Path) -> Self {
        let surfaced_topics = load_surfaced_topics(session_dir);
        let recall_count = surfaced_topics.len();
        let cached_index = lessons::load_index(session_dir);
        Self {
            surfaced_topics,
            recall_count,
            cached_index,
        }
    }

    pub fn on_message(&mut self, session: &Session, msg: &Message, sender: &str, owner_name: &str) {
        // Refresh cache when new lessons are saved
        if sender == "bpmux" && msg.text.contains("lesson saved") {
            self.cached_index = session.storage.load_lessons();
        }

        if !msg.is_chat()
            || sender == owner_name
            || sender == "bpmux"
            || self.recall_count >= MAX_RECALL_NUDGES
        {
            return;
        }

        let msg_keywords = lessons::extract_keywords(&msg.text);
        if msg_keywords.is_empty() {
            return;
        }

        let mut matches: Vec<&lessons::AgentLesson> = self
            .cached_index
            .lessons
            .iter()
            .filter(|l| {
                !self.surfaced_topics.contains(&l.topic)
                    && lessons::extract_keywords(&l.topic)
                        .iter()
                        .any(|tk| msg_keywords.iter().any(|mk| mk == tk))
            })
            .collect();

        matches.sort_by(|a, b| {
            let a_time = a.last_reinforced.as_deref().unwrap_or(&a.established);
            let b_time = b.last_reinforced.as_deref().unwrap_or(&b.established);
            a_time.cmp(b_time)
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
                addressee: None,
                explicit_privacy: false,
            };
            let _ = session.broadcast(&nudge);
            self.surfaced_topics.insert(lesson.topic.clone());
            self.recall_count += 1;
            if let Ok(json) = serde_json::to_string(&self.surfaced_topics) {
                session.storage.save_state("surfaced-topics", &json);
            }
        }
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

fn load_surfaced_topics(session_dir: &Path) -> HashSet<String> {
    let path = session_dir.join("surfaced-topics.json");
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        HashSet::new()
    }
}
