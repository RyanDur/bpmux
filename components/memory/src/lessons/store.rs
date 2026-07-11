use std::path::Path;

use super::{AgentLesson, LessonIndex};
use crate::identity::sanitize_name;

pub fn add_lesson(
    data_dir: &Path,
    topic: &str,
    lesson_text: &str,
    author_role: &str,
    evidence: Vec<String>,
) -> Result<(), std::io::Error> {
    let mut index = load_index(data_dir);
    let existing = index
        .lessons
        .iter_mut()
        .find(|l| l.topic == topic && l.author_role == author_role);
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(existing) = existing {
        existing.lesson = lesson_text.to_string();
        existing.last_reinforced = Some(now.clone());
        existing.reinforcement_count += 1;
        existing.reinforcement_history.push(now);
        existing.evidence = evidence;
    } else {
        index.lessons.push(AgentLesson {
            topic: topic.to_string(),
            lesson: lesson_text.to_string(),
            author_role: author_role.to_string(),
            evidence,
            established: now,
            last_reinforced: None,
            reinforcement_count: 0,
            reinforcement_history: Vec::new(),
        });
    }
    save_index(data_dir, &index)
}

pub fn load_index(data_dir: &Path) -> LessonIndex {
    let path = data_dir.join("lessons.json");
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        LessonIndex::default()
    }
}

pub fn save_index(data_dir: &Path, index: &LessonIndex) -> Result<(), std::io::Error> {
    let path = data_dir.join("lessons.json");
    let tmp = data_dir.join("lessons.json.tmp");
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| std::io::Error::other(format!("serialize: {e}")))?;
    std::fs::write(&tmp, &json)?;
    std::fs::rename(tmp, path)
}

pub fn save_peer_index(
    data_dir: &Path,
    peer_name: &str,
    index: &LessonIndex,
) -> Result<(), std::io::Error> {
    let safe_name = sanitize_name(peer_name);
    let path = data_dir.join(format!("lessons-peer-{safe_name}.json"));
    let tmp = data_dir.join(format!("lessons-peer-{safe_name}.json.tmp"));
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| std::io::Error::other(format!("serialize: {e}")))?;
    std::fs::write(&tmp, &json)?;
    std::fs::rename(tmp, path)
}

pub fn load_peer_index(data_dir: &Path, peer_name: &str) -> LessonIndex {
    let safe_name = sanitize_name(peer_name);
    let path = data_dir.join(format!("lessons-peer-{safe_name}.json"));
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        LessonIndex::default()
    }
}

pub fn list_peer_names(data_dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(data_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(peer) = name
                .strip_prefix("lessons-peer-")
                .and_then(|s| s.strip_suffix(".json"))
            {
                names.push(peer.to_string());
            }
        }
    }
    names.sort();
    names
}
