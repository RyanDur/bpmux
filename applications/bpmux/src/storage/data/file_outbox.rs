use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::FileStorage;
use persist::PersistError;
use source_sync::QueuedEvent;

pub(super) fn default_outbox_path() -> Result<PathBuf, std::io::Error> {
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

pub(crate) fn load_outbox_from_disk(path: &Path) -> Result<VecDeque<QueuedEvent>, PersistError> {
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

fn persist_outbox_to_disk(path: &Path, events: &[QueuedEvent]) -> Result<(), PersistError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(events)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

impl source_sync::OutboxRepo for FileStorage {
    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        let path = self.resolve_outbox_path()?;
        load_outbox_from_disk(&path)
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        let path = self.resolve_outbox_path()?;
        persist_outbox_to_disk(&path, events)
    }
}
