use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;

use crate::board::RefreshDriver;
use crate::bridge::tracker::ConditionalRunEntry;
use crate::ci::{CiEvent, CiState};
use crate::github::RateLimitObserver;
use crate::orchestrator::stores::{
    PinsAction, PinsBroadcastListener, PinsStore, ReactionsAction, ReactionsBroadcastListener,
    ReactionsStore, RosterAction, RosterBroadcastListener, RosterStore, StoreReduceOutcome,
    apply_pins, apply_reactions, apply_roster,
};
use crate::session;
use crate::transport::ci_state_store::CiStateStore;

pub trait CiPollFetch: Send + Sync + 'static {
    fn list_latest_run_on_main_conditional(
        &self,
        prior_etag: Option<&str>,
    ) -> impl Future<Output = Result<ConditionalRunEntry, String>> + Send;

    fn rate_limit_observer(&self) -> RateLimitObserver;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PollKind {
    Ci,
    Ingest,
    Hydration,
}

#[derive(Clone, Debug)]
pub enum OrchestratorAction {
    StartCiPolling,
    StopCiPolling,
    CiUpdated(CiSnapshot),
    StartIngestPolling,
    StopIngestPolling,
    StartHydrationPolling,
    StopHydrationPolling,
    Reactions(ReactionsAction),
    Pins(PinsAction),
    Roster(RosterAction),
}

pub type CiAction = OrchestratorAction;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiSnapshot {
    pub state: CiState,
    pub run_id: Option<u64>,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
    pub conclusion: &'static str,
}

impl CiSnapshot {
    pub fn to_ci_event(&self) -> CiEvent {
        CiEvent {
            state: self.state,
            sha: self.head_sha.clone(),
            title: self.display_title.clone(),
            run_id: self.run_id,
        }
    }
}

#[derive(Default)]
pub struct CiStore {
    inner: Mutex<CiStoreState>,
}

#[derive(Default, Clone)]
struct CiStoreState {
    last_seen: Option<(Option<u64>, &'static str)>,
    snapshot: Option<CiSnapshot>,
}

impl CiStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Option<CiSnapshot> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.snapshot.clone()
    }

    pub fn last_seen(&self) -> Option<(Option<u64>, &'static str)> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.last_seen
    }

    fn apply(&self, action: &OrchestratorAction) -> CiReduceOutcome {
        match action {
            OrchestratorAction::CiUpdated(snap) => {
                let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                let key = (snap.run_id, snap.conclusion);
                let changed = state.last_seen != Some(key);
                state.last_seen = Some(key);
                state.snapshot = Some(snap.clone());
                if changed {
                    CiReduceOutcome::Changed
                } else {
                    CiReduceOutcome::Unchanged
                }
            }
            _ => CiReduceOutcome::Unchanged,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiReduceOutcome {
    Changed,
    Unchanged,
}

pub trait CiBroadcastListener: Send + Sync {
    fn on_changed(&self, snap: &CiSnapshot);
}

pub struct Orchestrator<T: CiPollFetch> {
    session: Arc<session::Session>,
    tracker: Option<Arc<T>>,
    refresh: Option<Arc<dyn RefreshDriver>>,
    ingest_interval: Duration,
    hydration_interval: Duration,
    ci_state_store: CiStateStore,
    parent_ct: CancellationToken,
    rate_limit_observer: RateLimitObserver,
    ci_store: Arc<CiStore>,
    reactions_store: Arc<ReactionsStore>,
    pins_store: Arc<PinsStore>,
    roster_store: Arc<RosterStore>,
    tokens: AsyncMutex<HashMap<PollKind, CancellationToken>>,
    listeners: Mutex<Vec<Arc<dyn CiBroadcastListener>>>,
    reactions_listeners: Mutex<Vec<Arc<dyn ReactionsBroadcastListener>>>,
    pins_listeners: Mutex<Vec<Arc<dyn PinsBroadcastListener>>>,
    roster_listeners: Mutex<Vec<Arc<dyn RosterBroadcastListener>>>,
}

impl<T: CiPollFetch> Orchestrator<T> {
    pub fn new(
        session: Arc<session::Session>,
        tracker: Option<Arc<T>>,
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
            pins_store: Arc::new(PinsStore::new()),
            roster_store: Arc::new(RosterStore::new()),
            tokens: AsyncMutex::new(HashMap::new()),
            listeners: Mutex::new(Vec::new()),
            reactions_listeners: Mutex::new(Vec::new()),
            pins_listeners: Mutex::new(Vec::new()),
            roster_listeners: Mutex::new(Vec::new()),
        }
    }

    pub fn reactions_store(&self) -> Arc<ReactionsStore> {
        Arc::clone(&self.reactions_store)
    }

    pub fn pins_store(&self) -> Arc<PinsStore> {
        Arc::clone(&self.pins_store)
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

    pub fn add_pins_listener(&self, listener: Arc<dyn PinsBroadcastListener>) {
        let mut l = self
            .pins_listeners
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

    pub async fn dispatch(self: &Arc<Self>, action: OrchestratorAction) {
        validate_and_log(&action);
        match action {
            OrchestratorAction::StartCiPolling => self.middleware_start_ci().await,
            OrchestratorAction::StopCiPolling => self.middleware_stop(PollKind::Ci).await,
            OrchestratorAction::CiUpdated(_) => self.middleware_ci_updated(action).await,
            OrchestratorAction::StartIngestPolling => self.middleware_start_ingest().await,
            OrchestratorAction::StopIngestPolling => self.middleware_stop(PollKind::Ingest).await,
            OrchestratorAction::StartHydrationPolling => self.middleware_start_hydration().await,
            OrchestratorAction::StopHydrationPolling => {
                self.middleware_stop(PollKind::Hydration).await
            }
            OrchestratorAction::Reactions(a) => self.middleware_reactions(a),
            OrchestratorAction::Pins(a) => self.middleware_pins(a),
            OrchestratorAction::Roster(a) => self.middleware_roster(a),
        }
    }

    fn middleware_reactions(&self, action: ReactionsAction) {
        let outcome = apply_reactions(&self.reactions_store, &action);
        if outcome != StoreReduceOutcome::Changed {
            return;
        }
        let channel = match &action {
            ReactionsAction::ReactionToggled { channel, .. } => channel.clone(),
        };
        let aggregated = self.reactions_store.aggregated_for_channel(&channel);
        let listeners = {
            let l = self
                .reactions_listeners
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            l.clone()
        };
        for listener in listeners {
            listener.on_changed(&channel, &aggregated);
        }
    }

    fn middleware_pins(&self, action: PinsAction) {
        let outcome = apply_pins(&self.pins_store, &action);
        if outcome != StoreReduceOutcome::Changed {
            return;
        }
        let channel = match &action {
            PinsAction::PinAdded { channel, .. } | PinsAction::PinRemoved { channel, .. } => {
                channel.clone()
            }
        };
        let pins = self.pins_store.list_for_channel(&channel);
        let listeners = {
            let l = self
                .pins_listeners
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            l.clone()
        };
        for listener in listeners {
            listener.on_changed(&channel, &pins);
        }
    }

    fn middleware_roster(&self, action: RosterAction) {
        let outcome = apply_roster(&self.roster_store, &action);
        if outcome != StoreReduceOutcome::Changed {
            return;
        }
        let members = self.roster_store.members();
        let listeners = {
            let l = self
                .roster_listeners
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            l.clone()
        };
        for listener in listeners {
            listener.on_changed(&members);
        }
    }

    pub async fn is_running(&self, kind: PollKind) -> bool {
        let tokens = self.tokens.lock().await;
        tokens.contains_key(&kind)
    }

    async fn spawn_lifecycle_proxy(&self, local_ct: CancellationToken, task_ct: CancellationToken) {
        let parent_clone = self.parent_ct.clone();
        let local_clone = local_ct.clone();
        let task_clone = task_ct.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = parent_clone.cancelled() => {}
                _ = local_clone.cancelled() => {}
            }
            task_clone.cancel();
        });
    }

    async fn middleware_start_ci(self: &Arc<Self>) {
        let Some(tracker) = self.tracker.as_ref().cloned() else {
            return;
        };
        let mut tokens = self.tokens.lock().await;
        if tokens.contains_key(&PollKind::Ci) {
            return;
        }
        let local_ct = CancellationToken::new();
        let task_ct = CancellationToken::new();
        self.spawn_lifecycle_proxy(local_ct.clone(), task_ct.clone())
            .await;
        crate::transport::ci_watcher::spawn::<T>(
            Arc::clone(self),
            tracker,
            self.ci_state_store.clone(),
            task_ct,
        );
        tokens.insert(PollKind::Ci, local_ct);
    }

    async fn middleware_start_ingest(self: &Arc<Self>) {
        let Some(refresh) = self.refresh.as_ref().cloned() else {
            return;
        };
        let mut tokens = self.tokens.lock().await;
        if tokens.contains_key(&PollKind::Ingest) {
            return;
        }
        let local_ct = CancellationToken::new();
        let task_ct = CancellationToken::new();
        self.spawn_lifecycle_proxy(local_ct.clone(), task_ct.clone())
            .await;
        let observer = self.rate_limit_observer.clone();
        let interval = self.ingest_interval;
        tokio::spawn(crate::bridge::run_ingest_poll_loop_cancellable(
            refresh,
            interval,
            Some(observer),
            task_ct,
        ));
        tokens.insert(PollKind::Ingest, local_ct);
    }

    async fn middleware_start_hydration(self: &Arc<Self>) {
        let Some(refresh) = self.refresh.as_ref().cloned() else {
            return;
        };
        let mut tokens = self.tokens.lock().await;
        if tokens.contains_key(&PollKind::Hydration) {
            return;
        }
        let local_ct = CancellationToken::new();
        let task_ct = CancellationToken::new();
        self.spawn_lifecycle_proxy(local_ct.clone(), task_ct.clone())
            .await;
        let observer = self.rate_limit_observer.clone();
        let interval = self.hydration_interval;
        tokio::spawn(crate::bridge::run_hydration_loop_cancellable(
            refresh, interval, observer, task_ct,
        ));
        tokens.insert(PollKind::Hydration, local_ct);
    }

    async fn middleware_stop(&self, kind: PollKind) {
        let mut tokens = self.tokens.lock().await;
        if let Some(token) = tokens.remove(&kind) {
            token.cancel();
        }
    }

    async fn middleware_ci_updated(&self, action: OrchestratorAction) {
        let outcome = self.ci_store.apply(&action);
        if outcome == CiReduceOutcome::Changed {
            let snap = match action {
                OrchestratorAction::CiUpdated(s) => s,
                _ => return,
            };
            let listeners = {
                let l = self.listeners.lock().unwrap_or_else(|e| e.into_inner());
                l.clone()
            };
            for listener in listeners {
                listener.on_changed(&snap);
            }
        }
    }
}

fn validate_and_log(action: &OrchestratorAction) {
    let label = match action {
        OrchestratorAction::StartCiPolling => "StartCiPolling",
        OrchestratorAction::StopCiPolling => "StopCiPolling",
        OrchestratorAction::CiUpdated(_) => "CiUpdated",
        OrchestratorAction::StartIngestPolling => "StartIngestPolling",
        OrchestratorAction::StopIngestPolling => "StopIngestPolling",
        OrchestratorAction::StartHydrationPolling => "StartHydrationPolling",
        OrchestratorAction::StopHydrationPolling => "StopHydrationPolling",
        OrchestratorAction::Reactions(_) => "Reactions",
        OrchestratorAction::Pins(_) => "Pins",
        OrchestratorAction::Roster(_) => "Roster",
    };
    crate::session::diag::log_diag(&format!("orchestrator: dispatch {label}"));
}
