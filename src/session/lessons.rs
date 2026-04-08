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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LessonIndex {
    pub corrections: Vec<CorrectionEntry>,
}

#[derive(Debug, Clone)]
pub struct TopicGroup {
    pub topic: String,
    pub corrections: Vec<CorrectionEntry>,
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
    let mut groups: HashMap<String, Vec<CorrectionEntry>> = HashMap::new();
    for correction in &index.corrections {
        let topic = if correction.keywords.is_empty() {
            "general".to_string()
        } else {
            correction.keywords[..correction.keywords.len().min(3)].join(" ")
        };
        groups.entry(topic).or_default().push(correction.clone());
    }
    let mut result: Vec<TopicGroup> = groups
        .into_iter()
        .map(|(topic, corrections)| TopicGroup { topic, corrections })
        .collect();
    result.sort_by(|a, b| b.corrections.len().cmp(&a.corrections.len()));
    result
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
