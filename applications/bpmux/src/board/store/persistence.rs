use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use super::super::domain::Story;
use super::{BoardStore, PersistError};

pub fn default_store_path() -> Result<PathBuf, std::io::Error> {
    if let Ok(p) = std::env::var("BPMUX_BOARD_STORE_PATH")
        && !p.is_empty()
    {
        return Ok(PathBuf::from(p));
    }
    let base = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(base.join(".bpmux").join("board-store.json"))
}

impl BoardStore {
    pub fn load_from_disk(path: &Path) -> Result<Self, PersistError> {
        let store = Self::new();
        if !path.exists() {
            return Ok(store);
        }
        let bytes = std::fs::read(path)?;
        if bytes.is_empty() {
            return Ok(store);
        }
        let stories: HashMap<u64, Story> = serde_json::from_slice(&bytes)?;
        let mut w = store.stories.write().unwrap_or_else(|e| e.into_inner());
        *w = stories;
        drop(w);
        Ok(store)
    }

    pub fn persist_to_disk(&self, path: &Path) -> Result<(), PersistError> {
        let snapshot: HashMap<u64, Story> = {
            let r = self.stories.read().unwrap_or_else(|e| e.into_inner());
            r.clone()
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(&snapshot)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, path)?;
        self.dirty.store(false, Ordering::Release);
        Ok(())
    }

    pub fn load_from_storage(
        storage: &dyn crate::session::storage::Storage,
    ) -> Result<Self, PersistError> {
        let store = Self::new();
        let snapshot = storage.load_board_snapshot()?;
        let mut w = store.stories.write().unwrap_or_else(|e| e.into_inner());
        *w = snapshot;
        drop(w);
        Ok(store)
    }

    pub fn persist_to_storage(
        &self,
        storage: &dyn crate::session::storage::Storage,
    ) -> Result<(), PersistError> {
        let snapshot: HashMap<u64, Story> = {
            let r = self.stories.read().unwrap_or_else(|e| e.into_inner());
            r.clone()
        };
        storage.save_board_snapshot(&snapshot)?;
        self.dirty.store(false, Ordering::Release);
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }
}
