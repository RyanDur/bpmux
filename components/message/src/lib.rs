mod chat;
mod matching;
mod multi_depth;
mod state_delta;
mod state_snapshot;
mod subscription;
mod wire;

pub const GITHUB_BASE_URL: &str = "https://github.com";

pub use chat::{BROADCAST_ALL, Message, MessageType};
pub use matching::{addressee_matches, from_matches, to_reaches};
pub use multi_depth::{DeepDetail, MultiDepthPayload};
pub use state_delta::StateDeltaPayload;
pub use state_snapshot::{ReactionsChannelSnapshot, StateSnapshotPayload};
pub use subscription::payload_matches_filter;
pub use wire::{
    CiSnapshotWire, CiStateWire, IterationCloseBlockerPayload, PresenceWire, ReactionAggregateWire,
    RosterMemberWire, RosterRoleWire, StoryTransitionWire,
};

#[cfg(test)]
mod message_tests;
