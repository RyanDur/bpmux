use std::collections::HashSet;

use super::reducers::Stories;
use crate::product_management::story::{Story, StoryId, StoryKey, StoryKind};

pub fn all(stories: &Stories) -> Vec<Story> {
    stories.values().cloned().collect()
}

pub fn get(stories: &Stories, id: StoryId) -> Option<Story> {
    stories.get(&StoryKey::Saved(id)).cloned()
}

pub fn feature_ids(stories: &Stories) -> HashSet<StoryId> {
    stories
        .values()
        .filter(|story| matches!(story.story_type(), StoryKind::Feature))
        .map(|story| story.id)
        .collect()
}
