pub mod enums;
pub mod events;
mod gate_class;
pub mod key;
pub mod rework;
pub mod slice;
pub mod stage;
pub mod story_type;
pub mod verdict;

pub use enums::{Acceptor, Horizon, Tag, Work};
pub use gate_class::{gate_class_from_acceptor, gate_class_from_acceptor_and_story_type};
pub use key::{StoryId, StoryKey};
pub use rework::most_recent_needs_work;
pub use stage::Stage;
pub use story_type::{StoryKind, story_type_from_title};
pub use verdict::Verdict;

#[cfg(test)]
mod enums_tests;
#[cfg(test)]
mod gate_class_tests;
#[cfg(test)]
mod key_tests;
#[cfg(test)]
mod order_state_pin_tests;
#[cfg(test)]
mod rework_tests;
#[cfg(test)]
mod slice_tests;
#[cfg(test)]
mod story_type_tests;

use crate::epic::EpicId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastVerdict {
    pub verdict: Verdict,
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
    pub state: Work,
    pub filed_at: DateTime<Utc>,
    pub filed_by: String,
    pub body: Option<String>,
    pub free_comments: Option<Vec<FreeComment>>,
    pub verdicts: Option<Vec<CastVerdict>>,
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
    pub remote_updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub state_written_at: Option<DateTime<Utc>>,
}

impl Story {
    pub fn new_minimal(
        id: StoryId,
        title: String,
        state: Work,
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
            verdicts: None,
            body_etag: None,
            story_type,
            acceptor: None,
            parent_epic: None,
            tags: Vec::new(),
            remote_updated_at: None,
            state_written_at: None,
        }
    }

    pub fn key(&self) -> StoryKey {
        StoryKey::Saved(self.id)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn state(&self) -> Work {
        self.state.clone()
    }

    pub fn is_concluded(&self) -> bool {
        matches!(self.state, Work::Accepted | Work::Retired)
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

    pub fn verdicts(&self) -> Option<&[CastVerdict]> {
        self.verdicts.as_deref()
    }
}
