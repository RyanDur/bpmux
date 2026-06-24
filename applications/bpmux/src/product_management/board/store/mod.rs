use std::collections::HashMap;
use std::sync::RwLock;

use tokio::sync::broadcast;

use super::domain::{Story, StoryState};
use super::events::BoardEvent;

mod apply;
mod apply_events;
mod label_sync;
mod persist_error;

pub use persist_error::PersistError;

const SUBSCRIBE_BUFFER: usize = 1_000;

pub struct BoardStore {
    stories: RwLock<HashMap<u64, Story>>,
    event_tx: broadcast::Sender<BoardEvent>,
    #[cfg(test)]
    test_post_send_hook: std::sync::Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl BoardStore {
    pub fn new() -> Self {
        let (event_tx, _rx) = broadcast::channel(SUBSCRIBE_BUFFER);
        Self {
            stories: RwLock::new(HashMap::new()),
            event_tx,
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

    pub fn seed_all(&self, stories: Vec<Story>) {
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        for story in stories {
            w.insert(story.number, story);
        }
    }

    pub fn clear(&self) {
        self.stories
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    pub fn remove(&self, number: u64) -> Option<Story> {
        self.stories
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&number)
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
