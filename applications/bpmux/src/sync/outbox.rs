use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::{PersistError, QueuedEvent};

pub fn default_outbox_path() -> Result<PathBuf, std::io::Error> {
    if let Ok(p) = std::env::var("BPMUX_BOARD_OUTBOX_PATH")
        && !p.is_empty()
    {
        return Ok(PathBuf::from(p));
    }
    let base = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(base.join(".bpmux").join("board-outbox.json"))
}

pub fn load_outbox_from_disk(path: &Path) -> Result<VecDeque<QueuedEvent>, PersistError> {
    if !path.exists() {
        return Ok(VecDeque::new());
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Ok(VecDeque::new());
    }
    let entries: Vec<QueuedEvent> = serde_json::from_slice(&bytes)?;
    Ok(entries.into_iter().collect())
}

pub(crate) fn persist_outbox_to_disk(
    path: &Path,
    events: &[QueuedEvent],
) -> Result<(), PersistError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(events)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
