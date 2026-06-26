use std::sync::{Arc, Mutex};

use crate::store::kernel::{Listener, ListenerFuture, Reaction};
use crate::store::reactions::{self, ReactionsAction, ReactionsBroadcastListener, ReactionsState};
use crate::store::roster::{self, RosterAction, RosterBroadcastListener, RosterState};

#[derive(Default, Clone)]
pub struct AppState {
    pub roster: Arc<RosterState>,
    pub reactions: Arc<ReactionsState>,
}

#[derive(Clone)]
pub enum AppAction {
    Roster(RosterAction),
    Reactions(ReactionsAction),
}

impl From<RosterAction> for AppAction {
    fn from(action: RosterAction) -> Self {
        Self::Roster(action)
    }
}

impl From<ReactionsAction> for AppAction {
    fn from(action: ReactionsAction) -> Self {
        Self::Reactions(action)
    }
}

pub fn app_reduce(state: &AppState, action: &AppAction) -> AppState {
    match action {
        AppAction::Roster(action) => match roster::reduce(&state.roster, action) {
            Some(roster) => AppState {
                roster: Arc::new(roster),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::Reactions(action) => match reactions::reduce(&state.reactions, action) {
            Some(reactions) => AppState {
                reactions: Arc::new(reactions),
                ..state.clone()
            },
            None => state.clone(),
        },
    }
}

pub type RosterListeners = Arc<Mutex<Vec<Arc<dyn RosterBroadcastListener>>>>;
pub type ReactionsListeners = Arc<Mutex<Vec<Arc<dyn ReactionsBroadcastListener>>>>;

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
