use std::path::{Path, PathBuf};

use super::BoardPlacements;

pub fn placements_path(data_dir: &Path) -> PathBuf {
    data_dir.join("placements.json")
}

pub fn load(data_dir: &Path) -> BoardPlacements {
    let path = placements_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => BoardPlacements::default(),
    }
}

pub fn save(data_dir: &Path, placements: &BoardPlacements) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(data_dir)?;
    let path = placements_path(data_dir);
    let json = serde_json::to_string_pretty(placements)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}
