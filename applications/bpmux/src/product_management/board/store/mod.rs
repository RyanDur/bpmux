use std::sync::Arc;

use tokio::sync::broadcast;

use super::events::BoardEvent;
use crate::product_management::board::adapters::github::decode_iteration;
use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::{Story, StoryKey, StoryState, StoryType};
use crate::store::kernel::Store;

mod action;
mod apply;
mod middleware;
mod persist_error;
mod reducers;
mod state;

pub use action::BoardAction;
pub use persist_error::PersistError;
pub use state::BoardState;

const SUBSCRIBE_BUFFER: usize = 1_000;

pub struct BoardStore {
    store: Arc<Store<BoardState, BoardAction>>,
    event_tx: broadcast::Sender<BoardEvent>,
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

    pub fn seed_all(&self, stories: Vec<Story>) {
        self.store.dispatch(BoardAction::Seeded(stories));
    }

    pub fn clear(&self) {
        self.store.dispatch(BoardAction::Cleared);
    }

    pub fn remove(&self, number: u64) -> Option<Story> {
        let removed = self.store.select(|state| {
            state
                .stories
                .get(&StoryKey::Saved(number))
                .and_then(|item| item.as_story().cloned())
        });
        self.store.dispatch(BoardAction::Removed(number));
        removed
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BoardEvent> {
        self.event_tx.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.event_tx.receiver_count()
    }

    pub fn story(&self, number: u64) -> Option<Story> {
        self.store.select(|state| {
            state
                .stories
                .get(&StoryKey::Saved(number))
                .and_then(|item| item.as_story().cloned())
        })
    }

    pub fn all_stories(&self) -> Vec<Story> {
        self.store.select(|state| {
            state
                .stories
                .values()
                .filter_map(|item| item.as_story().cloned())
                .collect()
        })
    }

    pub fn outcomes(&self) -> Vec<Outcome> {
        self.store.select(|state| {
            state
                .stories
                .values()
                .filter_map(|item| item.as_outcome().cloned())
                .collect()
        })
    }

    pub fn iterations(&self) -> Vec<Iteration> {
        self.store.select(|state| {
            state
                .stories
                .values()
                .filter_map(|item| item.as_story())
                .filter(|story| story.story_type() == StoryType::Iteration)
                .map(decode_iteration)
                .collect()
        })
    }

    pub fn iteration(&self, number: u64) -> Option<Iteration> {
        self.store.select(|state| {
            state
                .stories
                .get(&StoryKey::Saved(number))
                .and_then(|item| item.as_story())
                .filter(|story| story.story_type() == StoryType::Iteration)
                .map(decode_iteration)
        })
    }

    pub fn stories_in_state(&self, wanted: StoryState) -> Vec<Story> {
        self.store.select(|state| {
            state
                .stories
                .values()
                .filter_map(|item| item.as_story())
                .filter(|story| story.state() == wanted)
                .cloned()
                .collect()
        })
    }

    pub fn len(&self) -> usize {
        self.store.select(|state| state.stories.len())
    }

    pub fn is_empty(&self) -> bool {
        self.store.select(|state| state.stories.is_empty())
    }
}

impl Default for BoardStore {
    fn default() -> Self {
        Self::new()
    }
}
