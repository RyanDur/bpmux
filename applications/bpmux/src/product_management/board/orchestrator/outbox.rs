use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::PersistError;
use crate::product_management::board::events::StoryEvent;

use super::QueuedEvent;

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

#[derive(Deserialize)]
#[serde(untagged)]
enum OutboxEntryOnDisk {
    NewShape(Box<QueuedEvent>),
    LegacyShape(Box<StoryEvent>),
}

impl From<OutboxEntryOnDisk> for QueuedEvent {
    fn from(disk: OutboxEntryOnDisk) -> QueuedEvent {
        match disk {
            OutboxEntryOnDisk::NewShape(q) => *q,
            OutboxEntryOnDisk::LegacyShape(event) => QueuedEvent::from_legacy(*event),
        }
    }
}

pub fn load_outbox_from_disk(path: &Path) -> Result<VecDeque<QueuedEvent>, PersistError> {
    if !path.exists() {
        return Ok(VecDeque::new());
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Ok(VecDeque::new());
    }
    let entries: Vec<OutboxEntryOnDisk> = serde_json::from_slice(&bytes)?;
    Ok(entries.into_iter().map(QueuedEvent::from).collect())
}

pub(super) fn persist_outbox_to_disk(
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
