use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StoryState {
    Icebox,
    Ready,
    InProgress,
    Delivered,
    ReadyForAccept,
    NeedsFixes,
    Rejected,
    Accepted,
}

impl StoryState {
    pub fn as_str(self) -> &'static str {
        match self {
            StoryState::Icebox => "icebox",
            StoryState::Ready => "ready",
            StoryState::InProgress => "in-progress",
            StoryState::Delivered => "delivered",
            StoryState::ReadyForAccept => "ready-for-accept",
            StoryState::NeedsFixes => "needs-fixes",
            StoryState::Rejected => "rejected",
            StoryState::Accepted => "accepted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StoryType {
    Feature,
    Bug,
    Chore,
    Epic,
}

pub fn story_type_from_labels_or_title(labels: &[String], title: &str) -> StoryType {
    for l in labels {
        match l.to_ascii_lowercase().as_str() {
            "feature" => return StoryType::Feature,
            "bug" => return StoryType::Bug,
            "chore" => return StoryType::Chore,
            "epic" => return StoryType::Epic,
            _ => {}
        }
    }
    let lower = title.to_lowercase();
    if lower.starts_with("[epic") {
        return StoryType::Epic;
    }
    if lower.starts_with("[bug") || lower.starts_with("bug:") {
        return StoryType::Bug;
    }
    if lower.starts_with("[chore")
        || lower.starts_with("[honest-plumbing")
        || lower.starts_with("[half-slice")
    {
        return StoryType::Chore;
    }
    StoryType::Feature
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tag {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gate {
    Filed,
    Pickup,
    Delivered,
    PassDeskCheck,
    FailDeskCheck,
    Rejected,
    Accepted,
}

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
}

impl Story {
    pub fn new_minimal(
        number: u64,
        title: String,
        state: StoryState,
        filed_at: DateTime<Utc>,
        filed_by: String,
    ) -> Self {
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
        }
    }
}

pub fn derive_epic_state(epic_explicit_state: StoryState, children: &[&Story]) -> StoryState {
    if epic_explicit_state == StoryState::Accepted {
        return StoryState::Accepted;
    }
    if children.is_empty() {
        return epic_explicit_state;
    }
    if children
        .iter()
        .any(|c| c.state == StoryState::NeedsFixes || c.state == StoryState::Rejected)
    {
        return StoryState::NeedsFixes;
    }
    if children.iter().any(|c| c.state == StoryState::InProgress) {
        return StoryState::InProgress;
    }
    let all_delivered_or_beyond = children.iter().all(|c| {
        matches!(
            c.state,
            StoryState::Delivered | StoryState::ReadyForAccept | StoryState::Accepted
        )
    });
    if all_delivered_or_beyond {
        return StoryState::Delivered;
    }
    epic_explicit_state
}

pub fn story_to_board_json(story: &Story) -> serde_json::Value {
    let mut labels: Vec<String> = Vec::new();
    labels.push(
        match story.story_type {
            StoryType::Feature => "feature",
            StoryType::Bug => "bug",
            StoryType::Chore => "chore",
            StoryType::Epic => "epic",
        }
        .to_string(),
    );
    for tag in &story.tags {
        match tag {
            Tag::Blocked => labels.push("blocked".to_string()),
        }
    }
    if let Some(epic) = story.parent_epic {
        labels.push(format!("parent-epic-#{epic}"));
    }
    serde_json::json!({
        "number": story.number,
        "title": story.title,
        "state": story.state.as_str(),
        "gh_state": if story.is_closed { "closed" } else { "open" },
        "labels": labels,
        "parent_epic": story.parent_epic,
        "assignees": Vec::<String>::new(),
        "created_at": story.filed_at.to_rfc3339(),
        "body": story.body.clone().unwrap_or_default(),
    })
}
