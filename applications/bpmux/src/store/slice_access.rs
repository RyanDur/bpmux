use std::sync::{Arc, Weak};

use super::app::{AppAction, AppState};
use super::kernel;
use super::{
    AggregatedReaction, CiBroadcastListener, CiSnapshot, ReactionsBroadcastListener,
    RosterBroadcastListener, RosterMember, Store, ci, reactions, roster,
};
use crate::product_management::board::{EpicReader, IterationReader, OutcomeReader};
use crate::product_management::epic::{self, Epic};
use crate::product_management::iteration::{self, Iteration};
use crate::product_management::outcome::{self, Outcome};

struct AppOutcomeReader {
    app: Weak<kernel::Store<AppState, AppAction>>,
}

impl OutcomeReader for AppOutcomeReader {
    fn outcomes(&self) -> Vec<Outcome> {
        self.app
            .upgrade()
            .map(|app| app.select(|state| outcome::all(&state.outcomes)))
            .unwrap_or_default()
    }
}

struct AppIterationReader {
    app: Weak<kernel::Store<AppState, AppAction>>,
}

impl IterationReader for AppIterationReader {
    fn iterations(&self) -> Vec<Iteration> {
        self.app
            .upgrade()
            .map(|app| app.select(|state| iteration::all(&state.iterations)))
            .unwrap_or_default()
    }
}

struct AppEpicReader {
    app: Weak<kernel::Store<AppState, AppAction>>,
}

impl EpicReader for AppEpicReader {
    fn epics(&self) -> Vec<Epic> {
        self.app
            .upgrade()
            .map(|app| app.select(|state| epic::all(&state.epics)))
            .unwrap_or_default()
    }
}

impl Store {
    pub fn outcome_reader(&self) -> Arc<dyn OutcomeReader> {
        Arc::new(AppOutcomeReader {
            app: Arc::downgrade(&self.app_store),
        })
    }

    pub fn iteration_reader(&self) -> Arc<dyn IterationReader> {
        Arc::new(AppIterationReader {
            app: Arc::downgrade(&self.app_store),
        })
    }

    pub fn epic_reader(&self) -> Arc<dyn EpicReader> {
        Arc::new(AppEpicReader {
            app: Arc::downgrade(&self.app_store),
        })
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
