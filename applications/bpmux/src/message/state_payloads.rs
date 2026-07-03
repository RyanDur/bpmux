use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "domain")]
pub enum StateDeltaPayload {
    #[serde(rename = "reactions")]
    Reactions {
        channel: String,
        #[serde(rename = "aggregated")]
        aggregated: Vec<ReactionAggregateWire>,
    },
    #[serde(rename = "ci")]
    Ci { snapshot: CiSnapshotWire },
    #[serde(rename = "ci_poll")]
    CiPoll { snapshot: CiSnapshotWire },
    #[serde(rename = "sync")]
    Sync { transition: SyncTransitionWire },
    #[serde(rename = "sync_poll")]
    SyncPoll { status: serde_json::Value },
    #[serde(rename = "story_transition")]
    StoryTransition { transition: StoryTransitionWire },
    #[serde(rename = "roster")]
    Roster { members: Vec<RosterMemberWire> },
    #[serde(rename = "iteration_close")]
    IterationClose {
        promoted: Vec<u64>,
        deleted_marker_id: Option<u64>,
    },
    #[serde(rename = "iteration_close_blocked")]
    IterationCloseBlocked {
        blockers: Vec<IterationCloseBlockerPayload>,
    },
    #[serde(rename = "board_invalidated")]
    BoardInvalidated { reason: String },
    #[serde(rename = "stories")]
    Stories { stories: serde_json::Value },
    #[serde(rename = "epics")]
    Epics { epics: serde_json::Value },
    #[serde(rename = "outcomes")]
    Outcomes { outcomes: serde_json::Value },
    #[serde(rename = "iterations")]
    Iterations { iterations: serde_json::Value },
    #[serde(rename = "roadmap")]
    Roadmap { roadmap: serde_json::Value },
}

impl StateDeltaPayload {
    pub fn to_subscription_event(&self, ts: &str) -> Option<(&'static str, serde_json::Value)> {
        match self {
            StateDeltaPayload::BoardInvalidated { reason } => {
                Some(("board_invalidated", serde_json::json!({ "reason": reason })))
            }
            StateDeltaPayload::Ci { snapshot } => Some((
                "ci_state_changed",
                serde_json::json!({
                    "conclusion": serde_json::to_value(snapshot.state)
                        .unwrap_or(serde_json::Value::Null),
                    "run_id": snapshot.run_id,
                    "head_sha": snapshot.head_sha,
                    "display_title": snapshot.display_title,
                    "url": crate::ci_link::github_actions_url(),
                }),
            )),
            StateDeltaPayload::Sync { transition } => Some((
                "sync_status_changed",
                serde_json::json!({
                    "status": transition.status,
                    "why": transition.why,
                    "pending_events": transition.pending_events,
                    "conflicts_pending": transition.conflicts_pending,
                }),
            )),
            StateDeltaPayload::StoryTransition { transition } => Some((
                "story_state_transition",
                serde_json::json!({
                    "story_id": transition.story_id,
                    "from_state": transition.from_state,
                    "target_state": transition.target_state,
                    "actor": transition.actor,
                    "ts": ts,
                }),
            )),
            _ => None,
        }
    }
}

pub fn payload_matches_filter(
    filter: Option<&serde_json::Map<String, serde_json::Value>>,
    payload: &serde_json::Value,
) -> bool {
    match filter {
        None => true,
        Some(criteria) => criteria
            .iter()
            .all(|(key, expected)| payload.get(key) == Some(expected)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IterationCloseBlockerPayload {
    pub id: u64,
    pub title: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StateSnapshotPayload {
    #[serde(default)]
    pub reactions: Vec<ReactionsChannelSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci: Option<CiSnapshotWire>,
    #[serde(default)]
    pub roster: Vec<RosterMemberWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReactionsChannelSnapshot {
    pub channel: String,
    pub aggregated: Vec<ReactionAggregateWire>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CiSnapshotWire {
    pub state: crate::ci::CiState,
    pub run_id: Option<u64>,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
    #[serde(default)]
    pub last_successful_poll_at: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RosterMemberWire {
    pub routing_id: String,
    pub name: String,
    pub identity: String,
    pub role: crate::store::RosterRole,
    pub presence: crate::store::Presence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiDepthPayload {
    pub summary: String,

    #[serde(default)]
    pub role_default: std::collections::HashMap<String, String>,

    pub deep: DeepDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeepDetail {
    Preloaded {
        text: String,
    },

    OnDemand {
        command: String,
        description: String,
    },
}
