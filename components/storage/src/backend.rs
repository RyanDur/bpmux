use std::path::Path;
use std::sync::Arc;

use super::instrumented::InstrumentedStorage;
use super::{FileStorage, SqliteStorage, Storage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    File,
    Sqlite,
}

impl StorageBackend {
    pub fn from_env_value(value: Option<&str>) -> Self {
        match value {
            Some("file") => Self::File,
            _ => Self::Sqlite,
        }
    }

    pub fn resolve() -> Self {
        Self::from_env_value(std::env::var("BPMUX_STORAGE_BACKEND").ok().as_deref())
    }

    pub fn build(self, dir: &Path) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite => match SqliteStorage::new(dir.to_path_buf()) {
                Ok(store) => {
                    inherit_file_history(dir);
                    Arc::new(InstrumentedStorage::new(store))
                }
                Err(e) => {
                    diag::log_diag(&format!(
                        "sqlite storage backend failed to open ({e}); falling back to file backend"
                    ));
                    Arc::new(InstrumentedStorage::new(FileStorage::new(
                        dir.to_path_buf(),
                    )))
                }
            },
            Self::File => Arc::new(InstrumentedStorage::new(FileStorage::new(
                dir.to_path_buf(),
            ))),
        }
    }
}

fn inherit_file_history(dir: &Path) {
    match super::migrate_file_log_to_sqlite(dir) {
        Ok(report) if !report.already_complete => diag::log_diag(&format!(
            "sqlite backend inherited {} message(s) from the jsonl log ({} ephemeral delta(s) dropped)",
            report.migrated, report.filtered
        )),
        Ok(_) => {}
        Err(e) => diag::log_diag(&format!(
            "jsonl history inheritance failed ({e}); the file log is untouched — run `bpmux migrate` to retry"
        )),
    }
    match super::migrate_routing_identity_to_agents(dir) {
        Ok(report) if !report.already_complete => diag::log_diag(&format!(
            "sqlite backend inherited {} routing identit(y/ies) from routing_identity.json",
            report.migrated
        )),
        Ok(_) => {}
        Err(e) => diag::log_diag(&format!(
            "routing-identity inheritance failed ({e}); run `bpmux migrate` to retry"
        )),
    }
}
