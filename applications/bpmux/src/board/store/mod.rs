use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::broadcast;

use super::domain::{Story, StoryState, Tag};
use super::events::BoardEvent;

#[derive(Debug)]
pub enum PersistError {
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Io(e) => write!(f, "i/o: {e}"),
            PersistError::Serde(e) => write!(f, "serde: {e}"),
        }
    }
}

impl std::error::Error for PersistError {}

impl From<std::io::Error> for PersistError {
    fn from(e: std::io::Error) -> Self {
        PersistError::Io(e)
    }
}

impl From<serde_json::Error> for PersistError {
    fn from(e: serde_json::Error) -> Self {
        PersistError::Serde(e)
    }
}

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

fn tag_from_label_string(s: &str) -> Option<Tag> {
    if s.eq_ignore_ascii_case("blocked") {
        Some(Tag::Blocked)
    } else {
        None
    }
}

fn sync_raw_labels_on_tag_added(story: &mut Story, label: &str) {
    if !story.raw_labels.iter().any(|l| l == label) {
        story.raw_labels.push(label.to_string());
    }
    story.parent_epic = crate::board::adapters::github::parent_epic_from_labels(&story.raw_labels);
    story.iteration_membership =
        crate::board::adapters::github::iteration_membership_from_labels(&story.raw_labels);
    story.is_current = crate::board::adapters::github::is_current_from_labels(&story.raw_labels);
    story.acceptor = crate::board::adapters::github::acceptor_from_labels(&story.raw_labels);
}

fn sync_raw_labels_on_tag_removed(story: &mut Story, label: &str) {
    story.raw_labels.retain(|l| l != label);
    story.parent_epic = crate::board::adapters::github::parent_epic_from_labels(&story.raw_labels);
    story.iteration_membership =
        crate::board::adapters::github::iteration_membership_from_labels(&story.raw_labels);
    story.is_current = crate::board::adapters::github::is_current_from_labels(&story.raw_labels);
    story.acceptor = crate::board::adapters::github::acceptor_from_labels(&story.raw_labels);
}

const SUBSCRIBE_BUFFER: usize = 1_000;

pub struct BoardStore {
    stories: RwLock<HashMap<u64, Story>>,
    event_tx: broadcast::Sender<BoardEvent>,
    dirty: AtomicBool,
    #[cfg(test)]
    test_post_send_hook: std::sync::Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl BoardStore {
    pub fn new() -> Self {
        let (event_tx, _rx) = broadcast::channel(SUBSCRIBE_BUFFER);
        Self {
            stories: RwLock::new(HashMap::new()),
            event_tx,
            dirty: AtomicBool::new(false),
            #[cfg(test)]
            test_post_send_hook: std::sync::Mutex::new(None),
        }
    }

    #[cfg(test)]
    pub fn set_test_post_send_hook<F>(&self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self
            .test_post_send_hook
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(Box::new(f));
    }

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

    pub fn seed_all(&self, stories: Vec<Story>) {
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        for story in stories {
            w.insert(story.number, story);
        }
        drop(w);
        self.dirty.store(true, Ordering::Release);
    }

    pub fn remove(&self, number: u64) -> Option<Story> {
        let removed = self
            .stories
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&number);
        if removed.is_some() {
            self.dirty.store(true, Ordering::Release);
        }
        removed
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BoardEvent> {
        self.event_tx.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.event_tx.receiver_count()
    }

    pub fn story(&self, number: u64) -> Option<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&number)
            .cloned()
    }

    pub fn all_stories(&self) -> Vec<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    pub fn stories_in_state(&self, state: StoryState) -> Vec<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|s| s.state == state)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.stories.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
    }
}

impl Default for BoardStore {
    fn default() -> Self {
        Self::new()
    }
}

mod apply;
