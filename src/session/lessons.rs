use crate::session::Message;
use crate::session::classifier::{MessageClass, classify};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "it", "to", "in", "of", "and", "or", "for", "on", "at", "by", "we",
    "i", "you", "that", "this", "with", "not", "but", "be", "are", "was", "do", "if", "so", "can",
    "my", "your", "have", "has", "had", "from", "just", "about", "would", "should", "could",
    "will", "all", "its", "also", "then", "than", "now", "how", "what", "when", "who", "why", "no",
    "yes", "up", "out", "them",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionEntry {
    pub timestamp: String,
    pub keywords: Vec<String>,
    pub evidence: Vec<String>,
    pub text_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLesson {
    pub topic: String,
    pub lesson: String,
    pub author_role: String,
    pub evidence: Vec<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LessonIndex {
    pub corrections: Vec<CorrectionEntry>,
    #[serde(default)]
    pub lessons: Vec<AgentLesson>,
}

#[derive(Debug, Clone)]
pub struct TopicGroup {
    pub topic: String,
    pub corrections: Vec<CorrectionEntry>,
    pub lessons: Vec<AgentLesson>,
}

pub fn extract_keywords(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2 && !STOP_WORDS.contains(w))
        .map(String::from)
        .collect()
}

pub fn build_index(messages: &[Message], owner_name: &str) -> LessonIndex {
    let mut index = LessonIndex::default();
    for msg in messages {
        if classify(msg, owner_name) == MessageClass::Correction {
            let keywords = extract_keywords(&msg.text);
            let preview: String = msg.text.chars().take(80).collect();
            index.corrections.push(CorrectionEntry {
                timestamp: msg.ts.to_rfc3339(),
                keywords,
                evidence: vec![msg.ts.to_rfc3339()],
                text_preview: preview,
            });
        }
    }
    index
}

pub fn group_by_topic(index: &LessonIndex) -> Vec<TopicGroup> {
    let mut correction_groups: HashMap<String, Vec<CorrectionEntry>> = HashMap::new();
    for correction in &index.corrections {
        let topic = if correction.keywords.is_empty() {
            "general".to_string()
        } else {
            correction.keywords[..correction.keywords.len().min(3)].join(" ")
        };
        correction_groups
            .entry(topic)
            .or_default()
            .push(correction.clone());
    }
    let mut all_topics: HashMap<String, TopicGroup> = HashMap::new();
    for (topic, corrections) in correction_groups {
        all_topics
            .entry(topic.clone())
            .or_insert_with(|| TopicGroup {
                topic,
                corrections: Vec::new(),
                lessons: Vec::new(),
            })
            .corrections = corrections;
    }
    for lesson in &index.lessons {
        let topic = lesson.topic.clone();
        all_topics
            .entry(topic.clone())
            .or_insert_with(|| TopicGroup {
                topic,
                corrections: Vec::new(),
                lessons: Vec::new(),
            })
            .lessons
            .push(lesson.clone());
    }
    let mut result: Vec<TopicGroup> = all_topics.into_values().collect();
    result.sort_by(|a, b| b.corrections.len().cmp(&a.corrections.len()));
    result
}

pub fn add_lesson(
    session_dir: &Path,
    topic: &str,
    lesson_text: &str,
    author_role: &str,
    evidence: Vec<String>,
) -> Result<(), std::io::Error> {
    let mut index = load_index(session_dir);
    let existing = index
        .lessons
        .iter_mut()
        .find(|l| l.topic == topic && l.author_role == author_role);
    if let Some(existing) = existing {
        existing.lesson = lesson_text.to_string();
        existing.timestamp = chrono::Utc::now().to_rfc3339();
        existing.evidence = evidence;
    } else {
        index.lessons.push(AgentLesson {
            topic: topic.to_string(),
            lesson: lesson_text.to_string(),
            author_role: author_role.to_string(),
            evidence,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }
    save_index(session_dir, &index)
}

pub fn load_index(session_dir: &Path) -> LessonIndex {
    let path = session_dir.join("lessons.json");
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        LessonIndex::default()
    }
}

pub fn save_index(session_dir: &Path, index: &LessonIndex) -> Result<(), std::io::Error> {
    let path = session_dir.join("lessons.json");
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| std::io::Error::other(format!("serialize: {e}")))?;
    std::fs::write(path, json)
}
