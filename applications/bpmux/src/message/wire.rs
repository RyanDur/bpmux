use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IterationCloseBlockerPayload {
    pub id: u64,
    pub title: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReactionAggregateWire {
    pub message_key: String,
    pub emoji: String,
    pub count: usize,
    pub users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncTransitionWire {
    pub status: String,
    pub why: String,
    pub pending_events: usize,
    pub conflicts_pending: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryTransitionWire {
    pub story_id: u64,
    pub from_state: String,
    pub target_state: String,
    pub actor: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiStateWire {
    #[default]
    Unknown,
    Green,
    Red,
    Running,
    QuotaExhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CiSnapshotWire {
    pub state: CiStateWire,
    pub run_id: Option<u64>,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
    #[serde(default)]
    pub last_successful_poll_at: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RosterRoleWire {
    Designer,
    Dev,
    Pm,
    Owner,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresenceWire {
    Online,
    Busy,
    Idle,
    Offline,
    Blocked,
    Help,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RosterMemberWire {
    pub routing_id: String,
    pub name: String,
    pub identity: String,
    pub role: RosterRoleWire,
    pub presence: PresenceWire,
}
