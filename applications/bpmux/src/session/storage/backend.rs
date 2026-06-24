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
            Some("sqlite") => Self::Sqlite,
            _ => Self::File,
        }
    }

    pub fn resolve() -> Self {
        Self::from_env_value(std::env::var("BPMUX_STORAGE_BACKEND").ok().as_deref())
    }

    pub fn build(self, dir: &Path) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite => match SqliteStorage::new(dir.to_path_buf()) {
                Ok(store) => Arc::new(InstrumentedStorage::new(store)),
                Err(e) => {
                    crate::session::diag::log_diag(&format!(
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
