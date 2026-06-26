use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;

use crate::product_management::board::{MutationId, RefreshDriver};
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

pub use action::{PollKind, StoreAction};
pub use ci::{
    BoxedFuture, CiAction, CiBroadcastListener, CiPollFetch, CiReduceOutcome, CiSessionBroadcaster,
    CiSnapshot, CiStateStoreWriter, CiStore,
};
pub use owner_park::{OwnerParkBroadcaster, owner_park_presence};
pub use reactions::{
    AggregatedReaction, ReactionsAction, ReactionsBroadcastListener, ReactionsSessionBroadcaster,
};
pub use roster::{
    Presence, RosterAction, RosterBroadcastListener, RosterMember, RosterRole,
    RosterSessionBroadcaster,
};
pub use sync::SyncAction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreReduceOutcome {
    Changed,
    Unchanged,
}

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
    tracker: Option<Arc<dyn CiPollFetch>>,
    refresh: Option<Arc<dyn RefreshDriver>>,
    ingest_interval: Duration,
    hydration_interval: Duration,
    ci_state_store: CiStateStore,
    parent_ct: CancellationToken,
    rate_limit_observer: crate::github::RateLimitObserver,
    ci_store: Arc<CiStore>,
    app_store: kernel::Store<app::AppState, app::AppAction>,
    tokens: AsyncMutex<HashMap<PollKind, CancellationToken>>,
    mutation_tokens: AsyncMutex<HashMap<MutationId, CancellationToken>>,
    listeners: Mutex<Vec<Arc<dyn CiBroadcastListener>>>,
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
        let reactions_listeners: app::ReactionsListeners = Arc::new(Mutex::new(Vec::new()));
        let app_store = kernel::Store::new(
            app::AppState::default(),
            Box::new(app::app_reduce),
            vec![
                Arc::new(app::ReactionsBroadcast::new(reactions_listeners.clone()))
                    as Arc<dyn kernel::Middleware<app::AppState, app::AppAction>>,
            ],
        );
        Self {
            session,
            tracker,
            refresh: None,
            ingest_interval: Duration::from_secs(60),
            hydration_interval: Duration::from_secs(15),
            ci_state_store,
            parent_ct,
            rate_limit_observer,
            ci_store: Arc::new(CiStore::new()),
            app_store,
            tokens: AsyncMutex::new(HashMap::new()),
            mutation_tokens: AsyncMutex::new(HashMap::new()),
            listeners: Mutex::new(Vec::new()),
            reactions_listeners,
        }
    }

    pub fn with_refresh(
        mut self,
        refresh: Arc<dyn RefreshDriver>,
        ingest_interval: Duration,
        hydration_interval: Duration,
    ) -> Self {
        self.refresh = Some(refresh);
        self.ingest_interval = ingest_interval;
        self.hydration_interval = hydration_interval;
        self
    }

    pub fn ci_store(&self) -> Arc<CiStore> {
        Arc::clone(&self.ci_store)
    }

    pub fn rate_limit_observer(&self) -> &crate::github::RateLimitObserver {
        &self.rate_limit_observer
    }

    pub fn session(&self) -> Arc<session::Session> {
        Arc::clone(&self.session)
    }

    pub fn add_ci_listener(&self, listener: Arc<dyn CiBroadcastListener>) {
        let mut listeners = self.listeners.lock().unwrap_or_else(|e| e.into_inner());
        listeners.push(listener);
    }

    pub fn spawn_action_drainer(
        self: &Arc<Self>,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<StoreAction>,
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
