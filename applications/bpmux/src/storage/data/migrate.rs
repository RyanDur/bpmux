use std::collections::HashMap;
use std::path::Path;

use super::{FileStorage, SqliteStorage, Storage};
use crate::routing::{UserRepo, short_name};
use crate::sync::PersistError;
use message::{Message, StateDeltaPayload};
use message_log::{LogError, MessageLog};

const MIGRATION_FLAG_KEY: &str = "migration:file_log_to_sqlite";
const MIGRATION_DONE: &str = "done";

const ROUTING_FLAG_KEY: &str = "migration:routing_identity_to_agents";
const ROUTING_JSON_FILE: &str = "routing_identity.json";

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
    #[error("write agents table: {0}")]
    Agents(#[from] PersistError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingMigrationReport {
    pub migrated: usize,
    pub already_complete: bool,
}

#[derive(Default, serde::Deserialize)]
struct LegacyRoutingIdentity {
    #[serde(default)]
    display_by_id: HashMap<String, String>,
    #[serde(default)]
    cursor_by_id: HashMap<String, String>,
}

fn is_ephemeral_stories(msg: &Message) -> bool {
    matches!(msg.state_delta, Some(StateDeltaPayload::Stories { .. }))
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
        if is_ephemeral_stories(msg) {
            filtered += 1;
            continue;
        }
        let mut msg = msg.clone();
        msg.ensure_id();
        target.append_message(&msg)?;
        migrated += 1;
    }
    target.save_state(MIGRATION_FLAG_KEY, MIGRATION_DONE);
    Ok(MigrationReport {
        migrated,
        filtered,
        already_complete: false,
    })
}

pub fn migrate_routing_identity_to_agents(
    data_dir: &Path,
) -> Result<RoutingMigrationReport, MigrationError> {
    let target = SqliteStorage::new(data_dir.to_path_buf())?;
    if target.load_state(ROUTING_FLAG_KEY).as_deref() == Some(MIGRATION_DONE) {
        return Ok(RoutingMigrationReport {
            migrated: 0,
            already_complete: true,
        });
    }
    let base = data_dir.parent().unwrap_or(data_dir);
    let legacy: LegacyRoutingIdentity = std::fs::read_to_string(base.join(ROUTING_JSON_FILE))
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();
    let mut migrated = 0;
    for (id, display) in &legacy.display_by_id {
        target.upsert_user(id, display, short_name(display))?;
        if let Some(ms) = legacy
            .cursor_by_id
            .get(id)
            .filter(|c| !c.is_empty())
            .and_then(|c| chrono::DateTime::parse_from_rfc3339(c).ok())
            .map(|dt| dt.timestamp_millis())
        {
            target.set_user_last_delivered(id, ms)?;
        }
        migrated += 1;
    }
    target.save_state(ROUTING_FLAG_KEY, MIGRATION_DONE);
    Ok(RoutingMigrationReport {
        migrated,
        already_complete: false,
    })
}
