pub mod enums;
pub mod key;
pub mod story_type;
pub mod transition;

pub use enums::{Acceptor, Gate, Horizon, Stage, StoryState, Tag};
pub use key::{StoryId, StoryKey};
pub use story_type::{StoryKind, story_type_from_title};
pub use transition::{
    StateTransition, is_backward, most_recent_backward_transition, replay_transitions,
};

#[cfg(test)]
mod enums_tests;
#[cfg(test)]
mod key_tests;
#[cfg(test)]
mod story_type_tests;

use crate::product_management::epic::EpicId;
use crate::product_management::iteration::IterationId;
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
pub struct Story {
    pub id: StoryId,
    pub title: String,
    pub state: StoryState,
    pub filed_at: DateTime<Utc>,
    pub filed_by: String,
    pub body: Option<String>,
    pub free_comments: Option<Vec<FreeComment>>,
    pub gate_comments: Option<Vec<GateComment>>,
    pub is_closed: bool,
    pub raw_labels: Vec<String>,
    #[serde(default)]
    pub body_etag: Option<String>,
    #[serde(default)]
    pub story_type: StoryKind,
    #[serde(default)]
    pub acceptor: Option<Acceptor>,
    #[serde(default)]
    pub parent_epic: Option<EpicId>,
    #[serde(default)]
    pub iteration_membership: Option<(IterationId, u32)>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub ready_order: Option<u32>,
}

impl Story {
    pub fn new_minimal(
        id: StoryId,
        title: String,
        state: StoryState,
        filed_at: DateTime<Utc>,
        filed_by: String,
    ) -> Self {
        let raw_labels = match state {
            StoryState::Icebox => Vec::new(),
            other => vec![other.github_label().to_string()],
        };
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
            raw_labels,
            body_etag: None,
            story_type,
            acceptor: None,
            parent_epic: None,
            iteration_membership: None,
            tags: Vec::new(),
            ready_order: None,
        }
    }

    pub fn key(&self) -> StoryKey {
        StoryKey::Saved(self.id)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn state(&self) -> StoryState {
        self.state
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

    pub fn iteration_membership(&self) -> Option<(IterationId, u32)> {
        self.iteration_membership
    }

    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    pub fn ready_order(&self) -> Option<u32> {
        self.ready_order
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

    pub fn raw_labels(&self) -> &[String] {
        &self.raw_labels
    }

    pub fn free_comments(&self) -> Option<&[FreeComment]> {
        self.free_comments.as_deref()
    }

    pub fn gate_comments(&self) -> Option<&[GateComment]> {
        self.gate_comments.as_deref()
    }
}
