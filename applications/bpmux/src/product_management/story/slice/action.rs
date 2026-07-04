use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{Story, StoryId};

#[derive(Clone)]
pub enum StoryAction {
    Event(StoryEvent),
    Seeded(Vec<Story>),
    Removed(StoryId),
    Cleared,
}
