use super::BoardStore;
use crate::product_management::story::{BoardState, Story, StoryId, StoryKey};

impl BoardStore {
    pub fn story(&self, id: StoryId) -> Option<Story> {
        self.store
            .select(|state| state.get(&StoryKey::Saved(id)).cloned())
    }

    pub fn backing_story(&self, id: StoryId) -> Option<Story> {
        self.store
            .select(|state| state.get(&StoryKey::Saved(id)).cloned())
    }

    pub fn all_stories(&self) -> Vec<Story> {
        self.store.select(|state| state.values().cloned().collect())
    }

    pub fn stories_in_state(&self, wanted: BoardState) -> Vec<Story> {
        self.store.select(|state| {
            state
                .values()
                .filter(|story| story.state() == wanted)
                .cloned()
                .collect()
        })
    }

    pub fn len(&self) -> usize {
        self.store.select(|state| state.len())
    }

    pub fn is_empty(&self) -> bool {
        self.store.select(|state| state.is_empty())
    }
}
