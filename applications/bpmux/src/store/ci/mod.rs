use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;

use crate::bridge::tracker::ConditionalRunEntry;
use crate::ci::{CiEvent, CiState};
use crate::github::RateLimitObserver;
use crate::product_management::board::{MutationId, RefreshDriver};
use crate::session;
use crate::store::stores::{
    ReactionsBroadcastListener, ReactionsStore, RosterBroadcastListener, RosterStore,
};
use crate::transport::ci_state_store::CiStateStore;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait CiPollFetch: Send + Sync + 'static {
    fn list_latest_run_on_main_conditional<'a>(
        &'a self,
        prior_etag: Option<&'a str>,
    ) -> BoxedFuture<'a, Result<ConditionalRunEntry, String>>;

    fn rate_limit_observer(&self) -> RateLimitObserver;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiSnapshot {
    pub state: CiState,
    pub run_id: Option<u64>,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
    pub conclusion: &'static str,
    pub workflows: Vec<crate::ci::WorkflowSummary>,
}

impl CiSnapshot {
    pub fn to_ci_event(&self) -> CiEvent {
        CiEvent {
            state: self.state,
            sha: self.head_sha.clone(),
            title: self.display_title.clone(),
            run_id: self.run_id,
            workflows: self.workflows.clone(),
        }
    }
}

pub trait CiBroadcastListener: Send + Sync {
    fn on_changed(&self, snap: &CiSnapshot);
}

pub struct Store {
    session: Arc<session::Session>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    refresh: Option<Arc<dyn RefreshDriver>>,
    ingest_interval: Duration,
    hydration_interval: Duration,
    ci_state_store: CiStateStore,
    parent_ct: CancellationToken,
    rate_limit_observer: RateLimitObserver,
    ci_store: Arc<CiStore>,
    reactions_store: Arc<ReactionsStore>,
    roster_store: Arc<RosterStore>,
    tokens: AsyncMutex<HashMap<PollKind, CancellationToken>>,
    mutation_tokens: AsyncMutex<HashMap<MutationId, CancellationToken>>,
    listeners: Mutex<Vec<Arc<dyn CiBroadcastListener>>>,
    reactions_listeners: Mutex<Vec<Arc<dyn ReactionsBroadcastListener>>>,
    roster_listeners: Mutex<Vec<Arc<dyn RosterBroadcastListener>>>,
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
            reactions_store: Arc::new(ReactionsStore::new()),
            roster_store: Arc::new(RosterStore::new()),
            tokens: AsyncMutex::new(HashMap::new()),
            mutation_tokens: AsyncMutex::new(HashMap::new()),
            listeners: Mutex::new(Vec::new()),
            reactions_listeners: Mutex::new(Vec::new()),
            roster_listeners: Mutex::new(Vec::new()),
        }
    }

    pub fn reactions_store(&self) -> Arc<ReactionsStore> {
        Arc::clone(&self.reactions_store)
    }

    pub fn roster_store(&self) -> Arc<RosterStore> {
        Arc::clone(&self.roster_store)
    }

    pub fn add_reactions_listener(&self, listener: Arc<dyn ReactionsBroadcastListener>) {
        let mut l = self
            .reactions_listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        l.push(listener);
    }

    pub fn add_roster_listener(&self, listener: Arc<dyn RosterBroadcastListener>) {
        let mut l = self
            .roster_listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        l.push(listener);
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

    pub fn rate_limit_observer(&self) -> &RateLimitObserver {
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

mod action;
mod middleware;
mod middleware_mutation;
mod snapshot;
mod store;

use action::validate_and_log;
pub use action::{CiAction, PollKind, StoreAction};
pub use store::{CiReduceOutcome, CiStore};
