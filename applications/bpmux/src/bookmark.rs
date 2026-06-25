use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Bookmark {
    #[serde(default)]
    pub current_task: String,

    #[serde(default)]
    pub decisions: Vec<String>,

    #[serde(default)]
    pub open_questions: Vec<String>,

    #[serde(default)]
    pub next_step: String,

    #[serde(default)]
    pub notes: String,

    #[serde(default)]
    pub last_updated: String,
}

pub fn bookmark_path(session_dir: &Path, role: &str) -> PathBuf {
    let safe_role = crate::memory::identity::sanitize_name(role);
    session_dir.join(format!("bookmark-{safe_role}.json"))
}

pub fn load_bookmark(session_dir: &Path, role: &str) -> Bookmark {
    let path = bookmark_path(session_dir, role);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Bookmark::default(),
    }
}

pub fn save_bookmark(
    session_dir: &Path,
    role: &str,
    bookmark: &Bookmark,
) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(session_dir)?;
    let path = bookmark_path(session_dir, role);
    let json = serde_json::to_string_pretty(bookmark)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}
