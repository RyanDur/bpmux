use std::collections::HashSet;

use crate::AwarenessHost;
use memory::lessons;
use message::Message;

const MAX_RECALL_NUDGES: usize = 3;

pub struct RecallSystem {
    surfaced_topics: HashSet<String>,
    recall_count: usize,
    cached_index: lessons::LessonIndex,
}

impl RecallSystem {
    pub fn new(host: &dyn AwarenessHost) -> Self {
        let surfaced_topics = host.load_surfaced_topics();
        let recall_count = surfaced_topics.len();
        let cached_index = host.own_lesson_index();
        Self {
            surfaced_topics,
            recall_count,
            cached_index,
        }
    }

    pub fn on_message(
        &mut self,
        host: &dyn AwarenessHost,
        msg: &Message,
        sender: &str,
        owner_name: &str,
    ) {
        if sender == "bpmux" && msg.text.contains("lesson saved") {
            self.cached_index = host.load_lessons();
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
                message_type: message::MessageType::Nudge,
                to: Some(vec![sender.to_string()]),
                ..Message::default()
            };
            let _ = host.broadcast(&nudge);
            self.surfaced_topics.insert(lesson.topic.clone());
            self.recall_count += 1;
            let _ = host.save_surfaced_topics(&self.surfaced_topics);
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
