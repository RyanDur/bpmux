pub mod enums;
pub mod transition;

pub use enums::{Acceptor, Gate, Horizon, StoryState, StoryType, Tag, story_type_from_title};
pub use transition::{
    StateTransition, is_backward, most_recent_backward_transition, replay_transitions,
};

#[cfg(test)]
mod enums_tests;

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
    pub number: u64,
    pub title: String,
    pub state: StoryState,
    pub story_type: StoryType,
    pub tags: Vec<Tag>,
    pub filed_at: DateTime<Utc>,
    pub filed_by: String,
    pub body: Option<String>,
    pub free_comments: Option<Vec<FreeComment>>,
    pub gate_comments: Option<Vec<GateComment>>,
    pub is_closed: bool,
    pub parent_epic: Option<u64>,
    pub acceptance_test_path: Option<String>,
    pub raw_labels: Vec<String>,
    #[serde(default)]
    pub body_etag: Option<String>,
    #[serde(default)]
    pub iteration_membership: Option<(u64, u32)>,
    #[serde(default)]
    pub iteration_review_closed_by: Vec<String>,
    #[serde(default)]
    pub iteration_dev_ok_by: Vec<String>,
    #[serde(default)]
    pub horizon: Option<Horizon>,
    #[serde(default)]
    pub outcome_membership: Option<(u64, u32)>,
    #[serde(default)]
    pub is_current: bool,
}

impl Story {
    pub fn new_minimal(
        number: u64,
        title: String,
        state: StoryState,
        filed_at: DateTime<Utc>,
        filed_by: String,
    ) -> Self {
        let raw_labels = match state {
            StoryState::Icebox => Vec::new(),
            other => vec![other.github_label().to_string()],
        };
        Self {
            number,
            title,
            state,
            story_type: StoryType::Feature,
            tags: Vec::new(),
            filed_at,
            filed_by,
            body: None,
            free_comments: None,
            gate_comments: None,
            is_closed: false,
            parent_epic: None,
            acceptance_test_path: None,
            raw_labels,
            body_etag: None,
            iteration_membership: None,
            iteration_review_closed_by: Vec::new(),
            iteration_dev_ok_by: Vec::new(),
            horizon: None,
            outcome_membership: None,
            is_current: false,
        }
    }
}
