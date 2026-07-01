use std::sync::{Arc, Mutex};

use super::{AppAction, AppState};
use crate::store::ci::{self, CiBroadcastListener};
use crate::store::kernel::{Listener, ListenerFuture, Reaction};
use crate::store::reactions::{self, ReactionsAction, ReactionsBroadcastListener};
use crate::store::roster::{self, RosterBroadcastListener};

pub type RosterListeners = Arc<Mutex<Vec<Arc<dyn RosterBroadcastListener>>>>;
pub type ReactionsListeners = Arc<Mutex<Vec<Arc<dyn ReactionsBroadcastListener>>>>;
pub type CiListeners = Arc<Mutex<Vec<Arc<dyn CiBroadcastListener>>>>;

pub struct RosterBroadcast {
    listeners: RosterListeners,
}

impl RosterBroadcast {
    pub fn new(listeners: RosterListeners) -> Self {
        Self { listeners }
    }
}

impl Listener<AppState, AppAction> for RosterBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.roster, &reaction.current.roster) {
            return Box::pin(async {});
        }
        let members = roster::members(&reaction.current.roster);
        let listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        Box::pin(async move {
            for listener in listeners {
                listener.on_changed(&members);
            }
        })
    }
}

pub struct ReactionsBroadcast {
    listeners: ReactionsListeners,
}

impl ReactionsBroadcast {
    pub fn new(listeners: ReactionsListeners) -> Self {
        Self { listeners }
    }
}

impl Listener<AppState, AppAction> for ReactionsBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        let AppAction::Reactions(ReactionsAction::ReactionToggled { channel, .. }) =
            &reaction.action
        else {
            return Box::pin(async {});
        };
        let channel = channel.clone();
        let aggregated = reactions::aggregated_for_channel(&reaction.current.reactions, &channel);
        let listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        Box::pin(async move {
            for listener in listeners {
                listener.on_changed(&channel, &aggregated);
            }
        })
    }
}

pub struct OutcomeBroadcast {
    session: Arc<crate::session::Session>,
    refresh: Arc<dyn crate::product_management::board::RefreshDriver>,
}

impl OutcomeBroadcast {
    pub fn new(
        session: Arc<crate::session::Session>,
        refresh: Arc<dyn crate::product_management::board::RefreshDriver>,
    ) -> Self {
        Self { session, refresh }
    }
}

impl Listener<AppState, AppAction> for OutcomeBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.outcomes, &reaction.current.outcomes) {
            return Box::pin(async {});
        }
        let session = Arc::clone(&self.session);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            crate::transport::rest::shared::broadcast_outcomes(&session, &refresh);
            crate::transport::rest::shared::broadcast_roadmap(&session, &refresh);
        })
    }
}

pub struct IterationBroadcast {
    session: Arc<crate::session::Session>,
    refresh: Arc<dyn crate::product_management::board::RefreshDriver>,
}

impl IterationBroadcast {
    pub fn new(
        session: Arc<crate::session::Session>,
        refresh: Arc<dyn crate::product_management::board::RefreshDriver>,
    ) -> Self {
        Self { session, refresh }
    }
}

impl Listener<AppState, AppAction> for IterationBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.iterations, &reaction.current.iterations) {
            return Box::pin(async {});
        }
        let session = Arc::clone(&self.session);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            crate::transport::rest::shared::broadcast_iterations(&session, &refresh);
            crate::transport::rest::shared::broadcast_outcomes(&session, &refresh);
            crate::transport::rest::shared::broadcast_roadmap(&session, &refresh);
        })
    }
}

pub struct EpicBroadcast {
    session: Arc<crate::session::Session>,
    refresh: Arc<dyn crate::product_management::board::RefreshDriver>,
}

impl EpicBroadcast {
    pub fn new(
        session: Arc<crate::session::Session>,
        refresh: Arc<dyn crate::product_management::board::RefreshDriver>,
    ) -> Self {
        Self { session, refresh }
    }
}

impl Listener<AppState, AppAction> for EpicBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.epics, &reaction.current.epics) {
            return Box::pin(async {});
        }
        let session = Arc::clone(&self.session);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            crate::transport::rest::shared::broadcast_epics(&session, &refresh);
        })
    }
}

pub struct CiBroadcast {
    listeners: CiListeners,
}

impl CiBroadcast {
    pub fn new(listeners: CiListeners) -> Self {
        Self { listeners }
    }
}

impl Listener<AppState, AppAction> for CiBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.ci, &reaction.current.ci) {
            return Box::pin(async {});
        }
        let Some(snapshot) = ci::snapshot(&reaction.current.ci) else {
            return Box::pin(async {});
        };
        let listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        Box::pin(async move {
            for listener in listeners {
                listener.on_changed(&snapshot);
            }
        })
    }
}
