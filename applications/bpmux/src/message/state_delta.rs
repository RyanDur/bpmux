use serde::{Deserialize, Serialize};

use super::wire::{
    CiSnapshotWire, IterationCloseBlockerPayload, ReactionAggregateWire, RosterMemberWire,
    StoryTransitionWire, SyncTransitionWire,
};

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
    Stories {
        stories: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<serde_json::Value>,
    },
    #[serde(rename = "epics")]
    Epics { epics: serde_json::Value },
    #[serde(rename = "outcomes")]
    Outcomes { outcomes: serde_json::Value },
    #[serde(rename = "iterations")]
    Iterations { iterations: serde_json::Value },
    #[serde(rename = "roadmap")]
    Roadmap { roadmap: serde_json::Value },
}
