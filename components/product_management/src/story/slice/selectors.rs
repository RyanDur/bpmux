use std::collections::HashSet;

use super::reducers::Stories;
use crate::story::{Story, StoryId, StoryKey, StoryKind};

pub fn all(stories: &Stories) -> Vec<Story> {
    stories.values().cloned().collect()
}

pub fn get(stories: &Stories, id: StoryId) -> Option<Story> {
    stories.get(&StoryKey::Saved(id)).cloned()
}

pub fn is_empty(stories: &Stories) -> bool {
    stories.is_empty()
}

pub fn feature_ids(stories: &Stories) -> HashSet<StoryId> {
    stories
        .values()
        .filter(|story| matches!(story.story_type(), StoryKind::Feature))
        .map(|story| story.id)
        .collect()
}
