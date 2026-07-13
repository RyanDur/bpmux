use std::sync::Arc;

use super::{
    AggregatedReaction, CiBroadcastListener, CiSnapshot, ReactionsBroadcastListener,
    RosterBroadcastListener, RosterMember, Select, Store, ci, reactions, roster,
};
use product_management::epic::{self, Epic};
use product_management::iteration::{self, Iteration};
use product_management::outcome::{self, Outcome};

impl Store {
    pub fn selector(&self) -> Select {
        Select::new(Arc::downgrade(&self.app_store))
    }

    pub fn outcomes(&self) -> Vec<Outcome> {
        self.app_store.select(|state| outcome::all(&state.outcomes))
    }

    pub fn iterations(&self) -> Vec<Iteration> {
        self.app_store
            .select(|state| iteration::all(&state.iterations))
    }

    pub fn epics(&self) -> Vec<Epic> {
        self.app_store.select(|state| epic::all(&state.epics))
    }

    pub fn ci_snapshot(&self) -> Option<CiSnapshot> {
        self.app_store.select(|state| ci::snapshot(&state.ci))
    }

    pub fn add_ci_listener(&self, listener: Arc<dyn CiBroadcastListener>) {
        self.ci_listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(listener);
    }

    pub fn roster_members(&self) -> Vec<RosterMember> {
        self.app_store
            .select(|state| roster::members(&state.roster))
    }

    pub fn reactions_channel_keys(&self) -> Vec<String> {
        self.app_store
            .select(|state| reactions::channel_keys(&state.reactions))
    }

    pub fn reactions_aggregated_for_channel(&self, channel: &str) -> Vec<AggregatedReaction> {
        self.app_store
            .select(|state| reactions::aggregated_for_channel(&state.reactions, channel))
    }

    pub fn reactions_is_present(
        &self,
        channel: &str,
        message_key: &str,
        emoji: &str,
        user: &str,
    ) -> bool {
        self.app_store.select(|state| {
            reactions::is_present(&state.reactions, channel, message_key, emoji, user)
        })
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
