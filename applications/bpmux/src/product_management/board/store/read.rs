use super::BoardStore;
use crate::product_management::story::{Story, StoryId, StoryKey, StoryState};

impl BoardStore {
    pub fn story(&self, id: StoryId) -> Option<Story> {
        self.store
            .select(|state| state.stories.get(&StoryKey::Saved(id)).cloned())
    }

    pub fn backing_story(&self, id: StoryId) -> Option<Story> {
        self.store
            .select(|state| state.stories.get(&StoryKey::Saved(id)).cloned())
    }

    pub fn all_stories(&self) -> Vec<Story> {
        self.store
            .select(|state| state.stories.values().cloned().collect())
    }

    pub fn stories_in_state(&self, wanted: StoryState) -> Vec<Story> {
        self.store.select(|state| {
            state
                .stories
                .values()
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
