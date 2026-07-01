use std::sync::{Arc, Mutex};

use crate::product_management::outcome::{self, OutcomeAction, OutcomeState};
use crate::store::ci::{self, CiAction, CiBroadcastListener, CiState};
use crate::store::kernel::{Listener, ListenerFuture, Reaction};
use crate::store::reactions::{self, ReactionsAction, ReactionsBroadcastListener, ReactionsState};
use crate::store::roster::{self, RosterAction, RosterBroadcastListener, RosterState};
use crate::store::sync::SyncAction;

#[derive(Default, Clone)]
pub struct AppState {
    pub roster: Arc<RosterState>,
    pub reactions: Arc<ReactionsState>,
    pub ci: Arc<CiState>,
    pub outcomes: Arc<OutcomeState>,
}

#[derive(Clone)]
pub enum AppAction {
    Roster(RosterAction),
    Reactions(ReactionsAction),
    Ci(CiAction),
    Sync(SyncAction),
    Outcome(OutcomeAction),
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

impl From<CiAction> for AppAction {
    fn from(action: CiAction) -> Self {
        Self::Ci(action)
    }
}

impl From<SyncAction> for AppAction {
    fn from(action: SyncAction) -> Self {
        Self::Sync(action)
    }
}

impl From<OutcomeAction> for AppAction {
    fn from(action: OutcomeAction) -> Self {
        Self::Outcome(action)
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
        AppAction::Ci(action) => match ci::reduce(&state.ci, action) {
            Some(ci) => AppState {
                ci: Arc::new(ci),
                ..state.clone()
            },
            None => state.clone(),
        },
        AppAction::Sync(_) => state.clone(),
        AppAction::Outcome(action) => match outcome::reduce(&state.outcomes, action) {
            Some(outcomes) => AppState {
                outcomes: Arc::new(outcomes),
                ..state.clone()
            },
            None => state.clone(),
        },
    }
}

pub(super) fn validate_and_log(action: &AppAction) {
    let label = match action {
        AppAction::Ci(CiAction::StartPolling) => "Ci.StartPolling",
        AppAction::Ci(CiAction::StopPolling) => "Ci.StopPolling",
        AppAction::Ci(CiAction::Updated(_)) => "Ci.Updated",
        AppAction::Sync(SyncAction::StartIngestPolling) => "Sync.StartIngestPolling",
        AppAction::Sync(SyncAction::StopIngestPolling) => "Sync.StopIngestPolling",
        AppAction::Sync(SyncAction::StartHydrationPolling) => "Sync.StartHydrationPolling",
        AppAction::Sync(SyncAction::StopHydrationPolling) => "Sync.StopHydrationPolling",
        AppAction::Sync(SyncAction::StartUpdateGithub { .. }) => "Sync.StartUpdateGithub",
        AppAction::Sync(SyncAction::StopUpdateGithub { .. }) => "Sync.StopUpdateGithub",
        AppAction::Sync(SyncAction::MutationConfirmed { .. }) => "Sync.MutationConfirmed",
        AppAction::Sync(SyncAction::MutationRolledBack { .. }) => "Sync.MutationRolledBack",
        AppAction::Reactions(_) => "Reactions",
        AppAction::Roster(_) => "Roster",
        AppAction::Outcome(_) => "Outcome",
    };
    crate::diag::log_diag(&format!("store: dispatch {label}"));
}

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
