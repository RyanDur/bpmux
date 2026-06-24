use crate::comprehension::classifier::classify;
use crate::session::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "it", "to", "in", "of", "and", "or", "for", "on", "at", "by", "we",
    "i", "you", "that", "this", "with", "not", "but", "be", "are", "was", "do", "if", "so", "can",
    "my", "your", "have", "has", "had", "from", "just", "about", "would", "should", "could",
    "will", "all", "its", "also", "then", "than", "now", "how", "what", "when", "who", "why", "no",
    "yes", "up", "out", "them",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceEntry {
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
    #[serde(alias = "timestamp")]
    pub established: String,
    #[serde(default)]
    pub last_reinforced: Option<String>,
    #[serde(default)]
    pub reinforcement_count: u32,
    #[serde(default)]
    pub reinforcement_history: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LessonIndex {
    #[serde(alias = "corrections")]
    pub experiences: Vec<ExperienceEntry>,
    #[serde(default)]
    pub lessons: Vec<AgentLesson>,
}

#[derive(Debug, Clone)]
pub struct TopicGroup {
    pub topic: String,
    pub experiences: Vec<ExperienceEntry>,
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
        if classify(msg, owner_name).is_experience() {
            let keywords = extract_keywords(&msg.text);
            let preview: String = msg.text.chars().take(80).collect();
            index.experiences.push(ExperienceEntry {
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
    let mut experience_groups: HashMap<String, Vec<ExperienceEntry>> = HashMap::new();
    for experience in &index.experiences {
        let topic = if experience.keywords.is_empty() {
            "general".to_string()
        } else {
            experience.keywords[..experience.keywords.len().min(3)].join(" ")
        };
        experience_groups
            .entry(topic)
            .or_default()
            .push(experience.clone());
    }
    let mut all_topics: HashMap<String, TopicGroup> = HashMap::new();
    for (topic, corrections) in experience_groups {
        all_topics
            .entry(topic.clone())
            .or_insert_with(|| TopicGroup {
                topic,
                experiences: Vec::new(),
                lessons: Vec::new(),
            })
            .experiences = corrections;
    }
    for lesson in &index.lessons {
        let topic = lesson.topic.clone();
        all_topics
            .entry(topic.clone())
            .or_insert_with(|| TopicGroup {
                topic,
                experiences: Vec::new(),
                lessons: Vec::new(),
            })
            .lessons
            .push(lesson.clone());
    }
    let mut result: Vec<TopicGroup> = all_topics.into_values().collect();
    result.sort_by_key(|g| std::cmp::Reverse(g.experiences.len()));
    result
}

mod store;
pub use store::*;
