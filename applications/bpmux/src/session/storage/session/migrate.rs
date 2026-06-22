use std::path::Path;

use super::{FileStorage, SqliteStorage, Storage};
use crate::session::log::LogError;
use crate::session::message::{Message, StateDeltaPayload};

const MIGRATION_FLAG_KEY: &str = "migration:file_log_to_sqlite";
const MIGRATION_DONE: &str = "done";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub migrated: usize,
    pub filtered: usize,
    pub already_complete: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("open sqlite target: {0}")]
    OpenTarget(#[from] rusqlite::Error),
    #[error("read/write message store: {0}")]
    Store(#[from] LogError),
    #[error("clear sqlite target: {0}")]
    Clear(std::io::Error),
}

fn is_ephemeral_board_snapshot(msg: &Message) -> bool {
    matches!(
        msg.state_delta,
        Some(StateDeltaPayload::BoardSnapshot { .. })
    )
}

pub fn migrate_file_log_to_sqlite(dir: &Path) -> Result<MigrationReport, MigrationError> {
    let target = SqliteStorage::new(dir.to_path_buf())?;
    if target.load_state(MIGRATION_FLAG_KEY).as_deref() == Some(MIGRATION_DONE) {
        return Ok(MigrationReport {
            migrated: 0,
            filtered: 0,
            already_complete: true,
        });
    }
    let source = FileStorage::new(dir.to_path_buf());
    let messages = source.read_messages()?;
    target.clear_messages().map_err(MigrationError::Clear)?;
    let mut migrated = 0;
    let mut filtered = 0;
    for msg in &messages {
        if is_ephemeral_board_snapshot(msg) {
            filtered += 1;
            continue;
        }
        target.append_message(msg)?;
        migrated += 1;
    }
    target.save_state(MIGRATION_FLAG_KEY, MIGRATION_DONE);
    Ok(MigrationReport {
        migrated,
        filtered,
        already_complete: false,
    })
}
