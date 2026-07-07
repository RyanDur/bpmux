use std::path::{Path, PathBuf};

use super::history::SharedHistory;

pub fn shared_history_path(session_dir: &Path) -> PathBuf {
    session_dir.join("shared-history.json")
}

pub fn load(session_dir: &Path) -> SharedHistory {
    let path = shared_history_path(session_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => SharedHistory::default(),
    }
}

pub fn save(session_dir: &Path, history: &SharedHistory) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(session_dir)?;
    let path = shared_history_path(session_dir);
    let json = serde_json::to_string_pretty(history)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}
