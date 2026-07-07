use super::{EventMeta, StoryEvent};
use crate::product_management::story::StoryId;

impl StoryEvent {
    pub fn meta(&self) -> &EventMeta {
        &self.meta
    }

    pub fn story_id(&self) -> StoryId {
        self.id
    }
}
