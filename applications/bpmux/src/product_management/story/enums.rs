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
    Outcome,
}

pub fn story_type_from_title(title: &str) -> StoryType {
    let lower = title.to_lowercase();
    if lower.starts_with("[epic") {
        return StoryType::Epic;
    }
    if lower.starts_with("[iteration") {
        return StoryType::Iteration;
    }
    if lower.starts_with("[outcome") {
        return StoryType::Outcome;
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
#[serde(rename_all = "lowercase")]
pub enum Horizon {
    Now,
    Next,
    Later,
}

impl Horizon {
    pub fn as_str(self) -> &'static str {
        match self {
            Horizon::Now => "now",
            Horizon::Next => "next",
            Horizon::Later => "later",
        }
    }

    pub fn band_label(self) -> &'static str {
        match self {
            Horizon::Now => "Now",
            Horizon::Next => "Next",
            Horizon::Later => "Later",
        }
    }

    pub fn from_cli(raw: &str) -> Option<Horizon> {
        match raw.to_ascii_lowercase().as_str() {
            "now" => Some(Horizon::Now),
            "next" => Some(Horizon::Next),
            "later" => Some(Horizon::Later),
            _ => None,
        }
    }

    pub const CLI_VALUES: &'static str = "now | next | later";
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
