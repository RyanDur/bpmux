use std::sync::Arc;

use super::{
    AggregatedReaction, ReactionsBroadcastListener, RosterBroadcastListener, RosterMember, Store,
    reactions, roster,
};

impl Store {
    pub fn roster_members(&self) -> Vec<RosterMember> {
        let mut members = Vec::new();
        self.app_store
            .read(&mut |state| members = roster::members(&state.roster));
        members
    }

    pub fn reactions_channel_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        self.app_store
            .read(&mut |state| keys = reactions::channel_keys(&state.reactions));
        keys
    }

    pub fn reactions_aggregated_for_channel(&self, channel: &str) -> Vec<AggregatedReaction> {
        let mut aggregated = Vec::new();
        self.app_store.read(&mut |state| {
            aggregated = reactions::aggregated_for_channel(&state.reactions, channel)
        });
        aggregated
    }

    pub fn add_reactions_listener(&self, listener: Arc<dyn ReactionsBroadcastListener>) {
        self.reactions_listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(listener);
    }

    pub fn add_roster_listener(&self, listener: Arc<dyn RosterBroadcastListener>) {
        self.roster_listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(listener);
    }
}
