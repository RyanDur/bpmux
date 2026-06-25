use std::sync::Arc;

mod reactions;
mod roster;

pub use reactions::{
    AggregatedReaction, ReactionsAction, ReactionsBroadcastListener, ReactionsStore,
};
pub use roster::{
    Presence, RosterAction, RosterBroadcastListener, RosterMember, RosterRole, RosterStore,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreReduceOutcome {
    Changed,
    Unchanged,
}

pub(super) fn apply_reactions(
    store: &Arc<ReactionsStore>,
    action: &ReactionsAction,
) -> StoreReduceOutcome {
    store.apply(action)
}

pub(super) fn apply_roster(store: &Arc<RosterStore>, action: &RosterAction) -> StoreReduceOutcome {
    store.apply(action)
}
