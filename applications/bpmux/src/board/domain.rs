use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::board::label_codec::LabelCodec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StoryState {
    Icebox,
    Ready,
    InProgress,
    DeskCheck,
    Delivered,
    Accepted,
}

impl StoryState {
    pub fn as_str(self) -> &'static str {
        match self {
            StoryState::Icebox => "icebox",
            StoryState::Ready => "ready",
            StoryState::InProgress => "in-progress",
            StoryState::DeskCheck => "desk-check",
            StoryState::Delivered => "delivered",
            StoryState::Accepted => "accepted",
        }
    }

    /// Parse the canonical state-string back into the enum. Accepts
    /// canonical forms (`in-progress`, `desk-check`) plus underscore
    /// variants the MCP tool callers commonly use (`in_progress`,
    /// `desk_check`). Returns None for any other string so callers
    /// can produce a clear named-cause error. Sister to #1367.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "icebox" => Some(StoryState::Icebox),
            "ready" => Some(StoryState::Ready),
            "in-progress" | "in_progress" => Some(StoryState::InProgress),
            "desk-check" | "desk_check" => Some(StoryState::DeskCheck),
            "delivered" => Some(StoryState::Delivered),
            "accepted" => Some(StoryState::Accepted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StoryType {
    Feature,
    Bug,
    Chore,
    Epic,
    Iteration,
}

pub fn story_type_from_labels_or_title(labels: &[String], title: &str) -> StoryType {
    for l in labels {
        match l.to_ascii_lowercase().as_str() {
            "feature" => return StoryType::Feature,
            "bug" => return StoryType::Bug,
            "chore" => return StoryType::Chore,
            "epic" => return StoryType::Epic,
            "iteration" => return StoryType::Iteration,
            _ => {}
        }
    }
    let lower = title.to_lowercase();
    if lower.starts_with("[epic") {
        return StoryType::Epic;
    }
    if lower.starts_with("[iteration") {
        return StoryType::Iteration;
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

/// One step in a story's state-transition history. Audit substrate is
/// derived from `gate_comments` via [`replay_transitions`]; there is
/// no dedicated GH persistence layer for transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: StoryState,
    pub to: StoryState,
    pub ts: DateTime<Utc>,
    pub actor: String,
    pub comment: Option<String>,
}

/// Replay a story's state-transitions from its gate-comment history.
/// Each `Gate` variant maps to a fixed (from, to) tuple, so the
/// gate-name is recoverable from the edge alone. Gate::Filed is the
/// creation event, not a transition, and is filtered out.
pub fn replay_transitions(gate_comments: &[GateComment]) -> Vec<StateTransition> {
    gate_comments
        .iter()
        .filter_map(|gc| {
            let (from, to) = match gc.gate {
                Gate::Filed => return None,
                Gate::Pickup => (StoryState::Ready, StoryState::InProgress),
                Gate::Delivered => (StoryState::InProgress, StoryState::DeskCheck),
                Gate::PassDeskCheck => (StoryState::DeskCheck, StoryState::Delivered),
                Gate::FailDeskCheck => (StoryState::DeskCheck, StoryState::InProgress),
                Gate::Rejected => (StoryState::Delivered, StoryState::Ready),
                Gate::Accepted => (StoryState::Delivered, StoryState::Accepted),
            };
            Some(StateTransition {
                from,
                to,
                ts: gc.ts,
                actor: gc.actor.clone(),
                comment: gc.payload.clone(),
            })
        })
        .collect()
}

/// Linear ordinal over the 6 column-aligned states; backward-edge
/// detection compares to-state's ordinal against from-state's.
fn state_ordinal(s: StoryState) -> u8 {
    match s {
        StoryState::Icebox => 0,
        StoryState::Ready => 1,
        StoryState::InProgress => 2,
        StoryState::DeskCheck => 3,
        StoryState::Delivered => 4,
        StoryState::Accepted => 5,
    }
}

pub fn is_backward(t: &StateTransition) -> bool {
    state_ordinal(t.to) < state_ordinal(t.from)
}

/// Returns the most-recent transition only if it's backward. Once a
/// later forward transition lands, this returns None — rework-context
/// stops surfacing as soon as the story moves forward again.
pub fn most_recent_backward_transition(
    transitions: &[StateTransition],
) -> Option<&StateTransition> {
    transitions.last().filter(|t| is_backward(t))
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
}

impl Story {
    pub fn new_minimal(
        number: u64,
        title: String,
        state: StoryState,
        filed_at: DateTime<Utc>,
        filed_by: String,
    ) -> Self {
        let state_label = match state {
            StoryState::Icebox => None,
            StoryState::Ready => Some("ready"),
            StoryState::InProgress => Some("in progress"),
            StoryState::DeskCheck => Some("desk-check"),
            StoryState::Delivered => Some("delivered"),
            StoryState::Accepted => Some("accepted"),
        };
        let raw_labels = state_label.map(|l| vec![l.to_string()]).unwrap_or_default();
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
    if children.iter().any(|c| c.state == StoryState::InProgress) {
        return StoryState::InProgress;
    }
    let all_delivered_or_beyond = children
        .iter()
        .all(|c| matches!(c.state, StoryState::Delivered | StoryState::Accepted));
    if all_delivered_or_beyond {
        return StoryState::Delivered;
    }
    epic_explicit_state
}

pub fn story_to_board_json<C: LabelCodec>(codec: &C, story: &Story) -> serde_json::Value {
    let mut labels: Vec<String> = Vec::new();
    labels.push(
        match story.story_type {
            StoryType::Feature => "feature",
            StoryType::Bug => "bug",
            StoryType::Chore => "chore",
            StoryType::Epic => "epic",
            StoryType::Iteration => "iteration",
        }
        .to_string(),
    );
    for tag in &story.tags {
        match tag {
            Tag::Blocked => labels.push("blocked".to_string()),
        }
    }
    if let Some(epic) = story.parent_epic {
        labels.push(codec.encode_parent_epic(epic));
    }
    for raw in &story.raw_labels {
        if codec.is_iteration_label_loose(raw) && !labels.iter().any(|l| l == raw) {
            labels.push(raw.clone());
        }
        if codec.is_ready_order_label(raw) && !labels.iter().any(|l| l == raw) {
            labels.push(raw.clone());
        }
    }
    // Derive rework-context from transition history so the renderer can
    // surface the ↩ glyph + tooltip without a per-story round-trip to
    // /api/stories/:n. Most stories have no backward edge; for those,
    // the field is absent from the payload (small wire cost).
    let rework_comment: Option<String> = story
        .gate_comments
        .as_ref()
        .map(|gates| replay_transitions(gates))
        .as_deref()
        .and_then(|ts| most_recent_backward_transition(ts).cloned())
        .and_then(|t| t.comment);
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
        "rework_comment": rework_comment,
    })
}
