use super::BoardStore;
use crate::product_management::story::{Story, StoryId, StoryKey, StoryState};

impl BoardStore {
    pub fn story(&self, id: StoryId) -> Option<Story> {
        self.store.select(|state| {
            state
                .stories
                .get(&StoryKey::Saved(id))
                .and_then(|item| item.as_story().cloned())
        })
    }

    pub fn backing_story(&self, id: StoryId) -> Option<Story> {
        self.store.select(|state| {
            state
                .stories
                .get(&StoryKey::Saved(id))
                .and_then(|item| item.backing_story().cloned())
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

    pub fn epics(&self) -> Vec<Story> {
        self.store.select(|state| {
            state
                .stories
                .values()
                .filter_map(|item| item.epic_story().cloned())
                .collect()
        })
    }

    pub fn epic(&self, id: StoryId) -> Option<Story> {
        self.store.select(|state| {
            state
                .stories
                .get(&StoryKey::Saved(id))
                .and_then(|item| item.epic_story().cloned())
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
