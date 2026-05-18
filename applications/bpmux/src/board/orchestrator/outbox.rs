//! Outbox persistence: on-disk shape of pending mutation events,
//! plus load/persist helpers used by `SyncOrchestrator` to keep the
//! pending-queue durable across restarts.
//!
//! The on-disk format is an untagged-enum bridge between the new
//! wrapped form (`{event, pre_state, ...}`) and the legacy bare-
//! BoardEvent form so existing outbox files keep loading. New writes
//! always use the wrapped form.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::board::events::BoardEvent;
use crate::board::store::PersistError;

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

/// #1195: outbox-entry on-disk shape. Untagged enum lets us read
/// both the new wrapped form (`{"event":{...},"pre_state":...}`)
/// and the legacy bare-BoardEvent form (`{"StoryCreated":{...}}`)
/// from the same file. New writes always use the wrapped form;
/// legacy entries get pre_state: None on read (degrade to
/// no-conflict-detection — the classifier returns None when
/// pre_state is missing). Both variants boxed so the enum stays
/// pointer-sized (clippy::large_enum_variant — `Story` inside
/// QueuedEvent makes the variant fat).
#[derive(Deserialize)]
#[serde(untagged)]
enum OutboxEntryOnDisk {
    NewShape(Box<QueuedEvent>),
    LegacyShape(Box<BoardEvent>),
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
