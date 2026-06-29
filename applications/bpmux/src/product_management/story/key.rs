use std::fmt;

use uuid::Uuid;

entity_id!(StoryId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoryKey {
    Pending(Uuid),
    Saved(u64),
}

impl StoryKey {
    pub fn saved_number(self) -> Option<u64> {
        match self {
            StoryKey::Saved(number) => Some(number),
            StoryKey::Pending(_) => None,
        }
    }
}

impl fmt::Display for StoryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoryKey::Saved(number) => write!(f, "#{number}"),
            StoryKey::Pending(_) => write!(f, "(unsaved)"),
        }
    }
}
