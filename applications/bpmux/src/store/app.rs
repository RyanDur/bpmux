use std::sync::{Arc, Mutex};

use crate::store::kernel::{Changed, Middleware, StoreApi};
use crate::store::reactions::{self, ReactionsAction, ReactionsBroadcastListener, ReactionsState};
use crate::store::roster::{self, RosterAction, RosterState};

#[derive(Default)]
pub struct AppState {
    pub roster: RosterState,
    pub reactions: ReactionsState,
}

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

pub fn app_reduce(state: &mut AppState, action: &AppAction) -> Changed {
    match action {
        AppAction::Roster(action) => roster::reduce(&mut state.roster, action),
        AppAction::Reactions(action) => reactions::reduce(&mut state.reactions, action),
    }
}

pub type ReactionsListeners = Arc<Mutex<Vec<Arc<dyn ReactionsBroadcastListener>>>>;

pub struct ReactionsBroadcast {
    listeners: ReactionsListeners,
}

impl ReactionsBroadcast {
    pub fn new(listeners: ReactionsListeners) -> Self {
        Self { listeners }
    }
}

impl Middleware<AppState, AppAction> for ReactionsBroadcast {
    fn handle(
        &self,
        api: &dyn StoreApi<AppState, AppAction>,
        next: &dyn Fn(AppAction),
        action: AppAction,
    ) {
        let channel = match &action {
            AppAction::Reactions(ReactionsAction::ReactionToggled { channel, .. }) => {
                Some(channel.clone())
            }
            _ => None,
        };
        next(action);
        let Some(channel) = channel else {
            return;
        };
        let mut aggregated = Vec::new();
        api.read(&mut |state: &AppState| {
            aggregated = reactions::aggregated_for_channel(&state.reactions, &channel);
        });
        let listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        for listener in listeners {
            listener.on_changed(&channel, &aggregated);
        }
    }
}
