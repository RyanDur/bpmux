use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StoryState {
    Icebox,
    Ready,
    InProgress,
    DeskCheck,
    Delivered,
    Acceptance,
    Accepted,
    Retired,
}

impl StoryState {
    pub fn as_str(self) -> &'static str {
        match self {
            StoryState::Icebox => "icebox",
            StoryState::Ready => "ready",
            StoryState::InProgress => "in-progress",
            StoryState::DeskCheck => "desk-check",
            StoryState::Delivered => "delivered",
            StoryState::Acceptance => "acceptance",
            StoryState::Accepted => "accepted",
            StoryState::Retired => "retired",
        }
    }

    pub fn github_label(self) -> &'static str {
        match self {
            StoryState::Icebox => "icebox",
            StoryState::Ready => "ready",
            StoryState::InProgress => "in progress",
            StoryState::DeskCheck => "desk-check",
            StoryState::Delivered => "delivered",
            StoryState::Acceptance => "acceptance",
            StoryState::Accepted => "accepted",
            StoryState::Retired => "retired",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "icebox" => Some(StoryState::Icebox),
            "ready" => Some(StoryState::Ready),
            "in-progress" | "in_progress" => Some(StoryState::InProgress),
            "desk-check" | "desk_check" => Some(StoryState::DeskCheck),
            "delivered" => Some(StoryState::Delivered),
            "acceptance" => Some(StoryState::Acceptance),
            "accepted" => Some(StoryState::Accepted),
            "retired" => Some(StoryState::Retired),
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
#[serde(rename_all = "lowercase")]
pub enum Acceptor {
    Team,
    Owner,
    Dev,
}

impl Acceptor {
    pub fn as_str(self) -> &'static str {
        match self {
            Acceptor::Team => "team",
            Acceptor::Owner => "owner",
            Acceptor::Dev => "dev",
        }
    }
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
pub struct StateTransition {
    pub from: StoryState,
    pub to: StoryState,
    pub ts: DateTime<Utc>,
    pub actor: String,
    pub comment: Option<String>,
}

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

fn state_ordinal(s: StoryState) -> u8 {
    match s {
        StoryState::Icebox => 0,
        StoryState::Ready => 1,
        StoryState::InProgress => 2,
        StoryState::DeskCheck => 3,
        StoryState::Delivered | StoryState::Acceptance => 4,
        StoryState::Accepted | StoryState::Retired => 5,
    }
}

pub fn is_backward(t: &StateTransition) -> bool {
    state_ordinal(t.to) < state_ordinal(t.from)
}

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
    #[serde(default)]
    pub iteration_membership: Option<(u64, u32)>,
    #[serde(default)]
    pub iteration_order: Option<u32>,
    #[serde(default)]
    pub iteration_review_closed_by: Vec<String>,
    #[serde(default)]
    pub is_current: bool,
    #[serde(default)]
    pub acceptor: Option<Acceptor>,
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
            iteration_order: None,
            iteration_review_closed_by: Vec::new(),
            is_current: false,
            acceptor: None,
        }
    }
}

pub fn derive_iteration_member_section(members: &[(u64, &str)]) -> String {
    if members.is_empty() {
        return "_(no members yet)_\n".to_string();
    }
    let mut out = String::new();
    for (number, title) in members {
        out.push_str(&format!("- #{number} — {title}\n"));
    }
    out
}

pub fn substitute_member_stories_section(body: &str, derived_content: &str) -> String {
    let header_marker = "## Member stories\n";
    let Some(header_pos) = body.find(header_marker) else {
        return body.to_string();
    };
    let section_start = header_pos + header_marker.len();
    let next_h2 = body[section_start..]
        .find("\n## ")
        .map(|i| section_start + i + 1)
        .unwrap_or(body.len());
    let trimmed = derived_content.trim_matches('\n');
    let mut out = String::with_capacity(body.len() + trimmed.len());
    out.push_str(&body[..section_start]);
    out.push('\n');
    out.push_str(trimmed);
    out.push_str("\n\n");
    out.push_str(&body[next_h2..]);
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EpicCounts {
    pub accepted_count: usize,
    pub retired_count: usize,
    pub active_count: usize,
    pub icebox_count: usize,
    pub total_count: usize,
}

impl EpicCounts {
    pub fn progress_denominator(&self) -> usize {
        self.accepted_count + self.active_count + self.icebox_count
    }
}

pub fn derive_epic_counts(children: &[&Story]) -> EpicCounts {
    let mut counts = EpicCounts {
        accepted_count: 0,
        retired_count: 0,
        active_count: 0,
        icebox_count: 0,
        total_count: children.len(),
    };
    for child in children {
        match child.state {
            StoryState::Accepted => counts.accepted_count += 1,
            StoryState::Retired => counts.retired_count += 1,
            StoryState::Icebox => counts.icebox_count += 1,
            StoryState::Ready
            | StoryState::InProgress
            | StoryState::DeskCheck
            | StoryState::Delivered
            | StoryState::Acceptance => counts.active_count += 1,
        }
    }
    counts
}

pub fn derive_epic_state(_epic_explicit_state: StoryState, children: &[&Story]) -> StoryState {
    if children.is_empty() || children.iter().all(|c| c.state == StoryState::Icebox) {
        return StoryState::Icebox;
    }
    let any_active = children.iter().any(|c| {
        matches!(
            c.state,
            StoryState::InProgress
                | StoryState::DeskCheck
                | StoryState::Delivered
                | StoryState::Acceptance
        )
    });
    if any_active {
        return StoryState::InProgress;
    }
    let non_retired: Vec<&&Story> = children
        .iter()
        .filter(|c| c.state != StoryState::Retired)
        .collect();
    if non_retired.is_empty() {
        return StoryState::Retired;
    }
    if non_retired.iter().all(|c| c.state == StoryState::Accepted) {
        return StoryState::Accepted;
    }
    if children.iter().any(|c| c.state == StoryState::Ready) {
        return StoryState::Ready;
    }
    StoryState::Icebox
}
