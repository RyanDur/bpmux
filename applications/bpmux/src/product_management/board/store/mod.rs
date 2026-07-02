use std::sync::Arc;

use tokio::sync::broadcast;

use super::events::StoryEvent;
use crate::product_management::story::slice::{Stories, StoryAction, StoryLabelMiddleware, reduce};
use crate::product_management::story::{Story, StoryId, StoryKey};
use crate::store::kernel::Store;

mod apply;
mod persist_error;
mod read;

pub use persist_error::PersistError;

const SUBSCRIBE_BUFFER: usize = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreChange {
    Event(StoryEvent),
    Resynced,
}

pub struct BoardStore {
    store: Arc<Store<Stories, StoryAction>>,
    event_tx: broadcast::Sender<StoreChange>,
    central_mirror: std::sync::OnceLock<Box<dyn Fn(StoryAction) + Send + Sync>>,
    #[cfg(test)]
    test_post_broadcast_hook: std::sync::Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl BoardStore {
    pub fn new() -> Self {
        let store = Store::new(Stories::default(), Box::new(reduce));
        store.add_middleware(Arc::new(StoryLabelMiddleware));
        let (event_tx, _rx) = broadcast::channel(SUBSCRIBE_BUFFER);
        Self {
            store,
            event_tx,
            central_mirror: std::sync::OnceLock::new(),
            #[cfg(test)]
            test_post_broadcast_hook: std::sync::Mutex::new(None),
        }
    }

    pub fn bind_central_mirror(&self, mirror: impl Fn(StoryAction) + Send + Sync + 'static) {
        let _ = self.central_mirror.set(Box::new(mirror));
    }

    pub(super) fn mirror_to_central(&self, action: StoryAction) {
        if let Some(mirror) = self.central_mirror.get() {
            mirror(action);
        }
    }

    #[cfg(test)]
    pub fn set_test_post_broadcast_hook<F>(&self, f: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self
            .test_post_broadcast_hook
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(Box::new(f));
    }

    pub fn seed_items(&self, items: Vec<Story>) {
        let changed = !items.is_empty();
        self.mirror_to_central(StoryAction::Seeded(items.clone()));
        self.store.dispatch(StoryAction::Seeded(items));
        if changed {
            let _ = self.event_tx.send(StoreChange::Resynced);
        }
    }

    pub fn seed_all(&self, stories: Vec<Story>) {
        self.seed_items(stories);
    }

    pub fn clear(&self) {
        self.mirror_to_central(StoryAction::Cleared);
        self.store.dispatch(StoryAction::Cleared);
        let _ = self.event_tx.send(StoreChange::Resynced);
    }

    pub fn remove(&self, id: StoryId) -> Option<Story> {
        let removed = self
            .store
            .select(|state| state.get(&StoryKey::Saved(id)).cloned());
        self.mirror_to_central(StoryAction::Removed(id));
        self.store.dispatch(StoryAction::Removed(id));
        if removed.is_some() {
            let _ = self.event_tx.send(StoreChange::Resynced);
        }
        removed
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StoreChange> {
        self.event_tx.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.event_tx.receiver_count()
    }
}

impl Default for BoardStore {
    fn default() -> Self {
        Self::new()
    }
}
