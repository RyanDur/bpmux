pub mod conflict;
pub mod enums;
pub mod events;
pub mod key;
pub mod rework;
pub(crate) mod slice;
pub mod stage;
pub mod story_type;

pub use enums::{Acceptor, BoardState, Gate, Horizon, Tag};
pub use key::{StoryId, StoryKey};
pub use rework::most_recent_needs_work;
pub use stage::Stage;
pub use story_type::{StoryKind, story_type_from_title};

#[cfg(test)]
mod enums_tests;
#[cfg(test)]
mod events_tests;
#[cfg(test)]
mod key_tests;
#[cfg(test)]
mod slice_tests;
#[cfg(test)]
mod story_type_tests;

use crate::product_management::epic::EpicId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateComment {
    pub gate: Gate,
    pub ts: DateTime<Utc>,
    pub actor: String,
    pub payload: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeComment {
    pub body: String,
    pub author: String,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IterationSlot {
    pub name: String,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Story {
    pub id: StoryId,
    pub title: String,
    pub state: BoardState,
    pub filed_at: DateTime<Utc>,
    pub filed_by: String,
    pub body: Option<String>,
    pub free_comments: Option<Vec<FreeComment>>,
    pub gate_comments: Option<Vec<GateComment>>,
    pub is_closed: bool,
    #[serde(default)]
    pub body_etag: Option<String>,
    #[serde(default)]
    pub story_type: StoryKind,
    #[serde(default)]
    pub acceptor: Option<Acceptor>,
    #[serde(default)]
    pub parent_epic: Option<EpicId>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub blocked_by: Option<StoryId>,
    #[serde(default)]
    pub ready_order: Option<u32>,
    #[serde(default)]
    pub iteration_slot: Option<IterationSlot>,
}

impl Story {
    pub fn new_minimal(
        id: StoryId,
        title: String,
        state: BoardState,
        filed_at: DateTime<Utc>,
        filed_by: String,
    ) -> Self {
        let story_type = story_type::story_type_from_title(&title);
        Self {
            id,
            title,
            state,
            filed_at,
            filed_by,
            body: None,
            free_comments: None,
            gate_comments: None,
            is_closed: false,
            body_etag: None,
            story_type,
            acceptor: None,
            parent_epic: None,
            tags: Vec::new(),
            blocked_by: None,
            ready_order: None,
            iteration_slot: None,
        }
    }

    pub fn key(&self) -> StoryKey {
        StoryKey::Saved(self.id)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn state(&self) -> BoardState {
        self.state.clone()
    }

    pub fn story_type(&self) -> StoryKind {
        self.story_type
    }

    pub fn acceptor(&self) -> Option<Acceptor> {
        self.acceptor
    }

    pub fn parent_epic(&self) -> Option<EpicId> {
        self.parent_epic
    }

    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    pub fn blocked_by(&self) -> Option<StoryId> {
        self.blocked_by
    }

    pub fn ready_order(&self) -> Option<u32> {
        self.ready_order
    }

    pub fn iteration_slot(&self) -> Option<&IterationSlot> {
        self.iteration_slot.as_ref()
    }

    pub fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    pub fn filed_at(&self) -> DateTime<Utc> {
        self.filed_at
    }

    pub fn filed_by(&self) -> &str {
        &self.filed_by
    }

    pub fn free_comments(&self) -> Option<&[FreeComment]> {
        self.free_comments.as_deref()
    }

    pub fn gate_comments(&self) -> Option<&[GateComment]> {
        self.gate_comments.as_deref()
    }
}
