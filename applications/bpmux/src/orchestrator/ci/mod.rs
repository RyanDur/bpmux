use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;

use crate::board::{MutationId, RefreshDriver};
use crate::bridge::tracker::ConditionalRunEntry;
use crate::ci::{CiEvent, CiState};
use crate::github::RateLimitObserver;
use crate::orchestrator::stores::{
    ReactionsAction, ReactionsBroadcastListener, ReactionsStore, RosterAction,
    RosterBroadcastListener, RosterStore,
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
    Roster(RosterAction),
    StartUpdateGithub {
        mutation_id: MutationId,
    },
    StopUpdateGithub {
        mutation_id: MutationId,
    },
    MutationConfirmed {
        mutation_id: MutationId,
    },
    MutationRolledBack {
        mutation_id: MutationId,
        server_story: Option<crate::board::domain::Story>,
    },
}

pub type CiAction = OrchestratorAction;

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
    pty_registry: Option<Arc<crate::room::LivePtyRegistry>>,
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
            pty_registry: None,
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

    pub fn build_snapshot(&self) -> crate::session::message::StateSnapshotPayload {
        use crate::session::message::{
            CiSnapshotWire, ReactionAggregateWire, ReactionsChannelSnapshot, RosterMemberWire,
            StateSnapshotPayload,
        };
        let reactions = self
            .reactions_store
            .channel_keys()
            .into_iter()
            .map(|channel| {
                let aggregated = self
                    .reactions_store
                    .aggregated_for_channel(&channel)
                    .into_iter()
                    .map(|a| ReactionAggregateWire {
                        message_key: a.message_key,
                        emoji: a.emoji,
                        count: a.count,
                        users: a.users,
                    })
                    .collect();
                ReactionsChannelSnapshot {
                    channel,
                    aggregated,
                }
            })
            .collect();
        let ci = self.ci_store.snapshot().map(|s| CiSnapshotWire {
            state: s.state,
            run_id: s.run_id,
            head_sha: s.head_sha,
            display_title: s.display_title,
        });
        let roster = self
            .roster_store
            .members()
            .into_iter()
            .map(|m| {
                let terminal_state = self
                    .pty_registry
                    .as_ref()
                    .and_then(|r| r.liveness(crate::transport::ws::short_name(&m.identity)))
                    .map(|alive| {
                        if alive {
                            crate::session::message::TerminalState::Alive
                        } else {
                            crate::session::message::TerminalState::Dead
                        }
                    });
                RosterMemberWire {
                    name: m.name,
                    identity: m.identity,
                    role: m.role,
                    presence: m.presence,
                    terminal_state,
                }
            })
            .collect();
        StateSnapshotPayload {
            reactions,
            ci,
            roster,
        }
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

    pub fn with_pty_registry(mut self, pty_registry: Arc<crate::room::LivePtyRegistry>) -> Self {
        self.pty_registry = Some(pty_registry);
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
        mut rx: tokio::sync::mpsc::UnboundedReceiver<OrchestratorAction>,
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
        OrchestratorAction::Roster(_) => "Roster",
        OrchestratorAction::StartUpdateGithub { .. } => "StartUpdateGithub",
        OrchestratorAction::StopUpdateGithub { .. } => "StopUpdateGithub",
        OrchestratorAction::MutationConfirmed { .. } => "MutationConfirmed",
        OrchestratorAction::MutationRolledBack { .. } => "MutationRolledBack",
    };
    crate::session::diag::log_diag(&format!("orchestrator: dispatch {label}"));
}

mod middleware;
