use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::product_management::board::RefreshDriver;
use crate::session;
use crate::transport::ci_state_store::CiStateStore;

pub mod ci;
pub mod kernel;
pub mod reactions;
pub mod roster;
pub mod sync;

mod action;
mod app;
mod dispatch;
mod owner_park;
mod select;
mod slice_access;
mod snapshot;

#[cfg(test)]
mod ci_tests;
#[cfg(test)]
mod kernel_tests;
#[cfg(test)]
mod listeners_tests;
#[cfg(test)]
mod stores_tests;
#[cfg(test)]
mod test_support;

pub use action::PollKind;
pub use app::AppAction;
pub use ci::{
    BoxedFuture, CiAction, CiBroadcastListener, CiPollFetch, CiSessionBroadcaster, CiSnapshot,
    CiStateStoreWriter,
};
pub use dispatch::Dispatch;
pub use owner_park::{OwnerParkBroadcaster, owner_park_presence};
pub use reactions::{
    AggregatedReaction, ReactionsAction, ReactionsBroadcastListener, ReactionsSessionBroadcaster,
};
pub use roster::{
    Presence, RosterAction, RosterBroadcastListener, RosterMember, RosterRole,
    RosterSessionBroadcaster,
};
#[cfg(test)]
pub(crate) use select::SeededStore;
pub use select::Select;
pub use sync::SyncAction;

pub(crate) fn broadcast_state_delta(
    session: &session::Session,
    payload: crate::message::StateDeltaPayload,
    label: &str,
) {
    let msg = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(payload),
        ..crate::message::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        crate::diag::log_diag(&format!("{label}: state-delta broadcast failed: {e}"));
    }
}

pub struct Store {
    session: Arc<session::Session>,
    parent_ct: CancellationToken,
    rate_limit_observer: crate::github::RateLimitObserver,
    ci_poll: Arc<ci::CiPoll>,
    sync_poll: Option<Arc<sync::SyncPoll>>,
    github_drain: Option<Arc<sync::GithubDrain>>,
    app_store: Arc<kernel::Store<app::AppState, app::AppAction>>,
    ci_listeners: app::CiListeners,
    roster_listeners: app::RosterListeners,
    reactions_listeners: app::ReactionsListeners,
}

impl Store {
    pub fn new(
        session: Arc<session::Session>,
        tracker: Option<Arc<dyn CiPollFetch>>,
        ci_state_store: CiStateStore,
        parent_ct: CancellationToken,
    ) -> Self {
        let rate_limit_observer = tracker
            .as_ref()
            .map(|t| t.rate_limit_observer())
            .unwrap_or_default();
        let roster_listeners: app::RosterListeners = Arc::new(Mutex::new(Vec::new()));
        let reactions_listeners: app::ReactionsListeners = Arc::new(Mutex::new(Vec::new()));
        let ci_listeners: app::CiListeners = Arc::new(Mutex::new(Vec::new()));
        let app_store = kernel::Store::new(app::AppState::default(), Box::new(app::app_reduce));
        app_store.add_listener(Arc::new(app::RosterBroadcast::new(
            roster_listeners.clone(),
        )));
        app_store.add_listener(Arc::new(app::ReactionsBroadcast::new(
            reactions_listeners.clone(),
        )));
        app_store.add_listener(Arc::new(app::CiBroadcast::new(ci_listeners.clone())));
        let ci_poll = Arc::new(ci::CiPoll::new(
            tracker.clone(),
            ci_state_store.clone(),
            session.clone(),
            parent_ct.clone(),
            rate_limit_observer.clone(),
        ));
        app_store.add_listener(ci_poll.clone());
        Self {
            session,
            parent_ct,
            rate_limit_observer,
            ci_poll,
            sync_poll: None,
            github_drain: None,
            app_store,
            ci_listeners,
            roster_listeners,
            reactions_listeners,
        }
    }

    pub fn with_refresh(
        mut self,
        refresh: Arc<dyn RefreshDriver>,
        ingest_interval: Duration,
        hydration_interval: Duration,
    ) -> Self {
        let sync_poll = Arc::new(sync::SyncPoll::new(
            refresh.clone(),
            self.rate_limit_observer.clone(),
            ingest_interval,
            hydration_interval,
            self.parent_ct.clone(),
        ));
        self.app_store.add_listener(sync_poll.clone());
        self.app_store
            .add_listener(Arc::new(app::OutcomeBroadcast::new(
                self.session.clone(),
                refresh.clone(),
            )));
        self.app_store
            .add_listener(Arc::new(app::IterationBroadcast::new(
                self.session.clone(),
                refresh.clone(),
            )));
        self.app_store
            .add_listener(Arc::new(app::EpicBroadcast::new(
                self.session.clone(),
                refresh.clone(),
            )));
        let github_drain = Arc::new(sync::GithubDrain::new(refresh, self.parent_ct.clone()));
        self.app_store.add_listener(github_drain.clone());
        self.sync_poll = Some(sync_poll);
        self.github_drain = Some(github_drain);
        self
    }

    pub fn rate_limit_observer(&self) -> &crate::github::RateLimitObserver {
        &self.rate_limit_observer
    }

    pub fn session(&self) -> Arc<session::Session> {
        Arc::clone(&self.session)
    }

    pub fn spawn_action_drainer(
        self: &Arc<Self>,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<AppAction>,
    ) -> tokio::task::JoinHandle<()> {
        let this = Arc::clone(self);
        let parent_ct = self.parent_ct.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = parent_ct.cancelled() => return,
                    maybe = rx.recv() => match maybe {
                        Some(action) => this.dispatch(action).await,
                        None => return,
                    }
                }
            }
        })
    }
}
