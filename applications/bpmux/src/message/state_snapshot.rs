use serde::{Deserialize, Serialize};

use super::wire::{CiSnapshotWire, ReactionAggregateWire, RosterMemberWire};

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
