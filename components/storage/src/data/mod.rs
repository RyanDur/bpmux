use std::path::PathBuf;

use persist::PersistError;

use super::Storage;
use user_routing::UserRecord;

pub struct FileStorage {
    dir: PathBuf,
    outbox_path: Option<PathBuf>,
}

impl FileStorage {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            outbox_path: None,
        }
    }

    pub fn with_outbox_path(mut self, path: PathBuf) -> Self {
        self.outbox_path = Some(path);
        self
    }

    fn resolve_outbox_path(&self) -> Result<PathBuf, std::io::Error> {
        if let Some(p) = &self.outbox_path {
            return Ok(p.clone());
        }
        file_outbox::default_outbox_path()
    }

    fn write_users(&self, records: &[UserRecord]) -> Result<(), PersistError> {
        let path = self.dir.join("users.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(records)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

mod file_lessons;
mod file_message_log;
mod file_outbox;
mod file_state;
mod file_surfaced_topics;
mod file_users;
pub use file_outbox::load_outbox_from_disk;

mod sqlite;
mod sqlite_engine;
mod sqlite_message_log;
mod sqlite_messages;
mod sqlite_meta;
mod sqlite_outbox;
mod sqlite_surfaced_topics;
mod sqlite_users;
pub use sqlite::*;

mod import;
mod migrate;
pub use import::*;
pub use migrate::*;
#[cfg(test)]
mod import_tests;
#[cfg(test)]
mod migrate_tests;
#[cfg(test)]
mod outbox_capture_tests;
#[cfg(test)]
mod outbox_format_tests;
#[cfg(test)]
mod sqlite_engine_behavior;
#[cfg(test)]
mod sqlite_store_behavior;
