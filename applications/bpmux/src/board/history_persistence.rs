use std::path::{Path, PathBuf};

use super::history::PlacementHistory;

pub fn placement_history_path(data_dir: &Path) -> PathBuf {
    data_dir.join("placement-history.json")
}

pub fn load(data_dir: &Path) -> PlacementHistory {
    let path = placement_history_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => PlacementHistory::default(),
    }
}

pub fn save(data_dir: &Path, history: &PlacementHistory) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(data_dir)?;
    let path = placement_history_path(data_dir);
    let json = serde_json::to_string_pretty(history)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}
