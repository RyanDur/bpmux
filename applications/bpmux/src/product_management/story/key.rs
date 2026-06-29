use std::fmt;

use uuid::Uuid;

entity_id!(StoryId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoryKey {
    Pending(Uuid),
    Saved(StoryId),
}

impl StoryKey {
    pub fn saved_id(self) -> Option<StoryId> {
        match self {
            StoryKey::Saved(id) => Some(id),
            StoryKey::Pending(_) => None,
        }
    }
}

impl fmt::Display for StoryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoryKey::Saved(id) => write!(f, "{id}"),
            StoryKey::Pending(_) => write!(f, "(unsaved)"),
        }
    }
}
