use std::sync::Arc;

use tokio::sync::broadcast;

use super::events::BoardEvent;
use crate::product_management::board::item::ProductItem;
use crate::product_management::story::{Story, StoryId, StoryKey};
use crate::store::kernel::Store;

mod action;
mod apply;
mod middleware;
mod persist_error;
mod read;
mod reducers;
mod state;

pub use action::BoardAction;
pub use persist_error::PersistError;
pub use state::BoardState;

const SUBSCRIBE_BUFFER: usize = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreChange {
    Event(BoardEvent),
    Resynced,
}

pub struct BoardStore {
    store: Arc<Store<BoardState, BoardAction>>,
    event_tx: broadcast::Sender<StoreChange>,
    #[cfg(test)]
    test_post_broadcast_hook: std::sync::Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl BoardStore {
    pub fn new() -> Self {
        let store = Store::new(BoardState::default(), Box::new(reducers::reduce));
        store.add_middleware(Arc::new(middleware::BoardLabelMiddleware));
        let (event_tx, _rx) = broadcast::channel(SUBSCRIBE_BUFFER);
        Self {
            store,
            event_tx,
            #[cfg(test)]
            test_post_broadcast_hook: std::sync::Mutex::new(None),
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

    pub fn seed_items(&self, items: Vec<ProductItem>) {
        let changed = !items.is_empty();
        self.store.dispatch(BoardAction::Seeded(items));
        if changed {
            let _ = self.event_tx.send(StoreChange::Resynced);
        }
    }

    pub fn seed_all(&self, stories: Vec<Story>) {
        self.seed_items(stories.iter().map(ProductItem::from_story).collect());
    }

    pub fn clear(&self) {
        self.store.dispatch(BoardAction::Cleared);
        let _ = self.event_tx.send(StoreChange::Resynced);
    }

    pub fn remove(&self, id: StoryId) -> Option<Story> {
        let removed = self.store.select(|state| {
            state
                .stories
                .get(&StoryKey::Saved(id))
                .and_then(|item| item.backing_story().cloned())
        });
        self.store.dispatch(BoardAction::Removed(id));
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
