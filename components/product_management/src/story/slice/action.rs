use crate::story::events::StoryEvent;
use crate::story::{Story, StoryId};

#[derive(Clone)]
pub enum StoryAction {
    Event(StoryEvent),
    Events(Vec<StoryEvent>),
    Seeded(Vec<Story>),
    Removed(StoryId),
    Cleared,
}
