use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::PersistError;
use crate::product_management::story::Story;
use crate::product_management::story::conflict::ConflictKind;
use crate::product_management::story::events::TolerantStoryEvent;

use super::{MutationId, QueuedEvent};

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
pub(crate) struct QueuedEventOnDisk {
    #[serde(default)]
    id: MutationId,
    event: TolerantStoryEvent,
    #[serde(default)]
    pre_state: Option<Story>,
    #[serde(default)]
    last_classification: Option<ConflictKind>,
}

impl QueuedEventOnDisk {
    pub(crate) fn into_queued(self) -> Option<QueuedEvent> {
        self.event.0.map(|event| QueuedEvent {
            id: self.id,
            event,
            pre_state: self.pre_state,
            last_classification: self.last_classification,
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OutboxEntryOnDisk {
    NewShape(Box<QueuedEventOnDisk>),
    LegacyShape(Box<TolerantStoryEvent>),
}

impl OutboxEntryOnDisk {
    fn into_queued(self) -> Option<QueuedEvent> {
        match self {
            OutboxEntryOnDisk::NewShape(q) => q.into_queued(),
            OutboxEntryOnDisk::LegacyShape(event) => event.0.map(QueuedEvent::from_legacy),
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
    Ok(entries
        .into_iter()
        .filter_map(OutboxEntryOnDisk::into_queued)
        .collect())
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
