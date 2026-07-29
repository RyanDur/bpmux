use serde::{Deserialize, Serialize};

use super::wire::{
    CiSnapshotWire, IterationCloseBlockerPayload, ReactionAggregateWire, RosterMemberWire,
    StoryTransitionWire,
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
    #[serde(rename = "reaction_updated")]
    ReactionUpdated {
        channel: String,
        message_key: String,
        emoji: String,
        count: u64,
        users: Vec<String>,
    },
    #[serde(rename = "roster_member_upserted")]
    RosterMemberUpserted { member: RosterMemberWire },
    #[serde(rename = "roster_member_removed")]
    RosterMemberRemoved { routing_id: String },
    #[serde(rename = "story_upserted")]
    StoryUpserted { story: serde_json::Value },
    #[serde(rename = "story_removed")]
    StoryRemoved { id: u64 },
    #[serde(rename = "story_order")]
    StoryOrder { ids: Vec<u64> },
    #[serde(rename = "epic_upserted")]
    EpicUpserted { epic: serde_json::Value },
    #[serde(rename = "epic_removed")]
    EpicRemoved { id: u64 },
    #[serde(rename = "epic_order")]
    EpicOrder { ids: Vec<u64> },
    #[serde(rename = "outcome_upserted")]
    OutcomeUpserted { outcome: serde_json::Value },
    #[serde(rename = "outcome_removed")]
    OutcomeRemoved { id: u64 },
    #[serde(rename = "outcome_order")]
    OutcomeOrder { ids: Vec<u64> },
    #[serde(rename = "iteration_upserted")]
    IterationUpserted { iteration: serde_json::Value },
    #[serde(rename = "iteration_removed")]
    IterationRemoved { id: u64 },
    #[serde(rename = "iteration_order")]
    IterationOrder { ids: Vec<u64> },
    #[serde(rename = "roadmap_card_upserted")]
    RoadmapCardUpserted { card: serde_json::Value },
    #[serde(rename = "roadmap_card_removed")]
    RoadmapCardRemoved { id: u64 },
    #[serde(rename = "roadmap_order")]
    RoadmapOrder { ids: Vec<u64> },
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
    #[serde(rename = "history")]
    History { messages: Vec<serde_json::Value> },
}
