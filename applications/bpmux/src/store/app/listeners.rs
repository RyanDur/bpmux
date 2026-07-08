use std::sync::{Arc, Mutex};

use super::{AppAction, AppState};
use crate::kernel::{DeliveryMode, Listener, ListenerFuture, Reaction};
use crate::store::ci::{self, CiBroadcastListener};
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

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
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

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
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

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}
