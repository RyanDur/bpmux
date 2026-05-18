use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::domain::{FreeComment, GateComment, Story};
use super::events::{BoardEvent, EventMeta};
use super::peer::PeerId;
use super::store::{BoardStore, PersistError};
use super::sync::{ConditionalBody, SourceAdapter, SyncError};

mod coalesce;
mod conflict;
mod drain;
mod outbox;
mod status;

use coalesce::{coalesce_iteration_label_events, stale_state_labels_for};
pub use conflict::{ConflictKind, ConflictResolveOrigin, classify_conflict};
pub use outbox::{default_outbox_path, load_outbox_from_disk};
pub use status::{
    DroppedEvent, SyncStatus, acknowledge_dropped, is_stale, is_startup_fail,
    mark_recovering_from_corruption, staleness_threshold_secs,
};
use status::{
    decrement_pending, record_conflicts_pending, record_dropped, record_error,
    record_ingest_success, record_push_success, set_pending_events,
};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MutationId(pub u64);

impl std::fmt::Display for MutationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug)]
pub enum DrainOneOutcome {
    Confirmed,
    RolledBack { server_story: Option<Box<Story>> },
    TransientError(SyncError),
    NotFound,
}

pub trait RefreshDriver: Send + Sync {
    fn refresh_story(&self, number: u64) -> BoxedFuture<'_, Result<Option<Story>, SyncError>>;
    fn refresh_all(&self) -> BoxedFuture<'_, Result<(), SyncError>>;
    fn refresh_all_since(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> BoxedFuture<'_, Result<(), SyncError>> {
        self.refresh_all()
    }
    fn mutate_sync(&self, event: BoardEvent) -> Result<(), SyncError>;
    fn drain_one_by_id(&self, id: MutationId) -> BoxedFuture<'_, DrainOneOutcome>;
    fn apply_server_rollback(&self, story: Story) -> BoxedFuture<'_, ()>;
    fn snapshot_pending_for_mutation(&self, id: MutationId) -> Option<u64>;
    fn issue_lock(&self, issue: u64) -> BoxedFuture<'_, Arc<tokio::sync::Mutex<()>>>;
    fn snapshot_status(&self) -> SyncStatus;
    fn acknowledge_dropped(&self) -> usize;
    fn mark_recovering_from_corruption(&self);
    fn drain_pending(&self) -> BoxedFuture<'_, Result<usize, SyncError>>;
    fn reconcile_dual_state_labels(&self) -> BoxedFuture<'_, Result<usize, SyncError>>;
    fn fetch_body(&self, number: u64) -> BoxedFuture<'_, Result<String, SyncError>>;
    fn fetch_body_conditional(
        &self,
        number: u64,
        _prior_etag: Option<String>,
    ) -> BoxedFuture<'_, Result<ConditionalBody, SyncError>> {
        Box::pin(async move {
            self.fetch_body(number)
                .await
                .map(|body| ConditionalBody::Modified { body, etag: None })
        })
    }
    fn fetch_free_comments(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<FreeComment>, SyncError>>;
    fn fetch_gate_comments(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<GateComment>, SyncError>>;
    fn create_story(&self, title: String, body: String) -> BoxedFuture<'_, Result<u64, SyncError>>;
    fn story(&self, number: u64) -> Option<Story>;
    fn all_stories(&self) -> Vec<Story>;
    /// Hand back the canonical BoardStore so handlers that need
    /// `&Arc<BoardStore>` (the read tools predating the trait) can
    /// share the same storage the orchestrator mutates. Added in #516
    /// slice 4 so server-side RPC dispatch can reuse the existing
    /// read handlers without forking them.
    fn store(&self) -> Arc<BoardStore>;
    /// Replace a story's body at the source-of-truth (#530). Bridges
    /// route body-edits through this trait method instead of using
    /// `gh issue edit` directly so the edit is observable to bpmux's
    /// BoardStore + WS broadcast cycle.
    fn update_body(&self, number: u64, body: String) -> BoxedFuture<'_, Result<(), SyncError>>;

    /// Replace a story's title at the source-of-truth (#1279). Mirrors
    /// `update_body` at the title-axis: pushes the new title through
    /// the adapter, then applies the returned Story to the canonical
    /// BoardStore via `BoardEvent::StoryRefreshed` so callers don't
    /// need a follow-up `refresh_story(N)` to observe the rename.
    fn update_title(&self, number: u64, title: String) -> BoxedFuture<'_, Result<(), SyncError>>;

    /// #1420 Slice 2: CI-state snapshot for the workflow gate's
    /// `check_ci_green` validator. Production orchestrator wires this
    /// from its `CiStateStore`; test/in-memory orchestrators return
    /// None (which the validator treats as `unknown`, refusing the
    /// transition). Default impl returns None so existing test
    /// substrates compile unchanged.
    fn ci_state(&self) -> Option<crate::workflow::CiSnapshot> {
        None
    }

    /// #1406 Slice 2: test-only race-window rendezvous hook for the
    /// gate-fire handlers (currently accept_story; sister-class fires
    /// can opt in). Production orchestrators no-op via this default
    /// impl. When env `BPMUX_TEST_RACE_WINDOW_REQUESTS=N` is set on
    /// the server subprocess, SyncOrchestrator builds a counted
    /// `RaceWindowBarrier(N)` and this method synchronizes N
    /// concurrent fires at the top of the handler — the
    /// Barrier-designated leader proceeds immediately; followers
    /// wait inside `arrive` for the leader's `complete()`. When
    /// followers wake the BoardStore reflects the leader's mutation,
    /// so substrate-validator `check_target_state_valid` returns
    /// `workflow-target-state-invalid` on the follower's check —
    /// canon-shape per #1385 Sc 9. Production single-orchestrator
    /// serialization is preserved bit-for-bit because the env var is
    /// never set on production servers.
    fn test_race_window_rendezvous(&self) -> BoxedFuture<'_, ()> {
        Box::pin(async {})
    }

    /// #1406 Slice 2 sister-method: leader signals critical-section
    /// completion so followers waiting in `test_race_window_rendezvous`
    /// can proceed and observe the mutated state. Called by gate-fire
    /// handlers AFTER the state-mutate (e.g., after
    /// `accept_via_canonical_store`). Production no-op via this
    /// default impl; SyncOrchestrator override calls
    /// `RaceWindowBarrier::complete()` when a barrier is configured.
    /// Idempotent — non-leaders calling it after the leader's call
    /// no-op cleanly.
    fn test_race_window_complete(&self) {}
}

/// #1406 Slice 2: leader-follower rendezvous barrier for test-only
/// e2e exercise of the gate-fire race-window (#1385 Sc 9). N
/// concurrent fires call `arrive().await`; the tokio Barrier
/// synchronizes them so none proceeds until all N have arrived. Then
/// the Barrier-designated leader returns first + does the critical
/// section (check + mutate); followers wait inside `arrive` until the
/// leader calls `complete()`. When followers wake, the BoardStore
/// reflects the leader's mutation, so the substrate-validator
/// (`check_target_state_valid`) returns 409
/// `workflow-target-state-invalid` on the follower's check — which is
/// exactly the canon-shape #1385 Sc 9 specifies.
///
/// `target = 0` makes the barrier a pass-through no-op (the
/// production code-path: env `BPMUX_TEST_RACE_WINDOW_REQUESTS` unset
/// → SyncOrchestrator builds no barrier → trait default no-op
/// `test_race_window_rendezvous`).
///
/// The leader-done signal uses a `tokio::sync::watch` channel — its
/// per-receiver version tracking sidesteps the lost-wakeup race
/// inherent to Notify (notify_waiters fires before follower
/// registers → notification dropped → follower deadlocks). watch
/// gives followers a `borrow()` view of the current value plus
/// `changed()` that resolves on any version bump beyond the
/// receiver's seen version, so a complete() that fires before the
/// follower's check is observable via the immediate borrow.
pub struct RaceWindowBarrier {
    target: usize,
    arrival_barrier: tokio::sync::Barrier,
    leader_done_tx: tokio::sync::watch::Sender<bool>,
    leader_done_rx: tokio::sync::watch::Receiver<bool>,
}

impl RaceWindowBarrier {
    pub fn new(target: usize) -> Self {
        let (tx, rx) = tokio::sync::watch::channel(false);
        Self {
            target,
            arrival_barrier: tokio::sync::Barrier::new(target.max(1)),
            leader_done_tx: tx,
            leader_done_rx: rx,
        }
    }

    pub async fn arrive(&self) {
        if self.target == 0 {
            return;
        }
        let is_leader = self.arrival_barrier.wait().await.is_leader();
        if is_leader {
            return;
        }
        let mut rx = self.leader_done_rx.clone();
        loop {
            if *rx.borrow_and_update() {
                return;
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
    }

    pub fn complete(&self) {
        if self.target == 0 {
            return;
        }
        let _ = self.leader_done_tx.send(true);
    }
}

/// #1195: outbox-entry wrapper carrying the pre-mutation snapshot
/// captured at queue-time. Required for divergence detection at
/// drain-time: comparing the event's intended mutation to the
/// canonical store's CURRENT state always says "already applied"
/// because `mutate()` applies locally before the outbox push (see
/// `feedback_mutate_sync_breaks_drain_time_idempotent_check`). The
/// pre_state lets the classifier ask "did the world change between
/// when I queued this and when I'm about to push it?"
///
/// `pre_state: None` means: legacy outbox entry from before #1195,
/// OR the story didn't exist in the store at queue-time (e.g.
/// pre-ingest). Either way, the classifier degrades to
/// no-conflict-detection on those entries (returns None / pushes
/// normally).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    #[serde(default)]
    pub id: MutationId,
    pub event: BoardEvent,
    #[serde(default)]
    pub pre_state: Option<Story>,
    /// #1195: classification carried-forward from prior drain tick
    /// so auto-resolve can detect "was conflict, now not". Reset
    /// every tick by re-running classify_conflict against the
    /// current canonical state.
    #[serde(default)]
    pub last_classification: Option<ConflictKind>,
}

impl QueuedEvent {
    pub fn new(event: BoardEvent, pre_state: Option<Story>) -> Self {
        Self {
            id: MutationId::default(),
            event,
            pre_state,
            last_classification: None,
        }
    }

    pub fn new_with_id(id: MutationId, event: BoardEvent, pre_state: Option<Story>) -> Self {
        Self {
            id,
            event,
            pre_state,
            last_classification: None,
        }
    }

    pub fn from_legacy(event: BoardEvent) -> Self {
        Self {
            id: MutationId::default(),
            event,
            pre_state: None,
            last_classification: None,
        }
    }
}

pub struct SyncOrchestrator<A: SourceAdapter + 'static> {
    pub(super) store: Arc<BoardStore>,
    pub(super) adapter: Arc<A>,
    pub(super) status: Arc<Mutex<SyncStatus>>,
    pub(super) pending: Arc<Mutex<VecDeque<QueuedEvent>>>,
    pub(super) outbox_path: Option<PathBuf>,
    /// #1289 Phase-1 step 4: Storage-trait sink for outbox persistence.
    /// When `Some`, used in preference to `outbox_path`. Set via
    /// `with_storage` constructor (sister of `with_outbox_path`).
    pub(super) storage: Option<Arc<dyn crate::session::storage::Storage>>,
    /// #1382: constructor-injected LabelCodec. Defaults to GhLabelCodec
    /// via the existing constructors (preserves 30+ call-site API);
    /// tests can swap a stub via the `with_codec` builder method to
    /// drive iteration_prefix_class / coalesce_iteration_label_events
    /// against arbitrary label syntax.
    pub(super) codec: Arc<dyn crate::board::label_codec::LabelCodec + Send + Sync>,
    pub(super) next_mutation_id: AtomicU64,
    pub(super) action_tx:
        Option<tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>>,
    pub(super) issue_locks:
        tokio::sync::Mutex<std::collections::HashMap<u64, Arc<tokio::sync::Mutex<()>>>>,
    /// #1420 Slice 2: shared CI-state store the workflow gate
    /// validator reads through `RefreshDriver::ci_state()`. Production
    /// server.rs wires this via `with_ci_state_store`; test-mode and
    /// in-memory orchestrators leave it None (gate then treats CI as
    /// `unknown` and refuses CI-gated transitions).
    pub(super) ci_state_store: Option<crate::transport::ci_state_store::CiStateStore>,
    /// #1406 Slice 2: counted rendezvous barrier for test-only e2e
    /// exercise of gate-fire race-window. Populated at startup only
    /// when env `BPMUX_TEST_RACE_WINDOW_REQUESTS` is set (test
    /// subprocess); None in production (never set on production
    /// servers). When Some, `test_race_window_rendezvous` awaits the
    /// barrier between substrate-validator check and state-mutate so
    /// the e2e test can observe the substrate-validator's
    /// concurrent-fire path.
    pub(super) race_window_barrier: Option<Arc<RaceWindowBarrier>>,
}

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn new(store: Arc<BoardStore>, adapter: Arc<A>) -> Self {
        Self {
            store,
            adapter,
            status: Arc::new(Mutex::new(SyncStatus::default())),
            pending: Arc::new(Mutex::new(VecDeque::new())),
            outbox_path: None,
            storage: None,
            codec: Arc::new(crate::board::adapters::gh_label_codec::GhLabelCodec),
            next_mutation_id: AtomicU64::new(1),
            action_tx: None,
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            ci_state_store: None,
            race_window_barrier: None,
        }
    }

    /// #1420 Slice 2: inject the shared CI-state store so the
    /// workflow gate's `check_ci_green` validator can read aggregate
    /// CI state through `RefreshDriver::ci_state()`. Production
    /// server.rs calls this between `Orchestrator::new` and
    /// `spawn_action_drainer`; test-mode orchestrators leave it
    /// unset (gate then refuses CI-gated transitions, which is the
    /// canonical-correct behavior — tests must opt in to a green
    /// snapshot via this builder).
    pub fn with_ci_state_store(
        mut self,
        store: crate::transport::ci_state_store::CiStateStore,
    ) -> Self {
        self.ci_state_store = Some(store);
        self
    }

    /// #1406 Slice 2: inject the test-only counted rendezvous barrier
    /// so the gate-fire handlers (currently accept_story) can
    /// interleave at the substrate-validator → state-mutate boundary
    /// for e2e race-window exercise. Production server.rs only calls
    /// this when env `BPMUX_TEST_RACE_WINDOW_REQUESTS=N` is set
    /// (test subprocess); production servers never set the env so
    /// this stays None + the trait's default no-op `arrive` makes
    /// the gate handler proceed without yielding.
    pub fn with_race_window_barrier(mut self, barrier: Arc<RaceWindowBarrier>) -> Self {
        self.race_window_barrier = Some(barrier);
        self
    }

    /// #1382: builder for stub-codec injection at test-time. Returns
    /// the same orchestrator with the codec field replaced; preserves
    /// every other field including pending-state from
    /// `with_outbox_path` / `with_storage` constructors. Production
    /// code never calls this — the default GhLabelCodec is the only
    /// runtime codec.
    pub fn with_codec(
        mut self,
        codec: Arc<dyn crate::board::label_codec::LabelCodec + Send + Sync>,
    ) -> Self {
        self.codec = codec;
        self
    }

    pub fn with_outbox_path(
        store: Arc<BoardStore>,
        adapter: Arc<A>,
        path: PathBuf,
    ) -> Result<Self, PersistError> {
        let pending = load_outbox_from_disk(&path)?;
        let next = pending.iter().map(|e| e.id.0).max().unwrap_or(0) + 1;
        Ok(Self {
            store,
            adapter,
            status: Arc::new(Mutex::new(SyncStatus {
                pending_events: pending.len(),
                ..SyncStatus::default()
            })),
            pending: Arc::new(Mutex::new(pending)),
            outbox_path: Some(path),
            storage: None,
            codec: Arc::new(crate::board::adapters::gh_label_codec::GhLabelCodec),
            next_mutation_id: AtomicU64::new(next),
            action_tx: None,
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            ci_state_store: None,
            race_window_barrier: None,
        })
    }

    /// #1289 Phase-1 step 4: construct with Storage-trait sink for
    /// outbox persistence. Sister of `with_outbox_path` at storage-DI
    /// axis. When this constructor is used, `persist_outbox` routes
    /// through `storage.save_outbox` instead of `persist_outbox_to_disk`,
    /// and initial pending events come from `storage.load_outbox`.
    pub fn with_storage(
        store: Arc<BoardStore>,
        adapter: Arc<A>,
        storage: Arc<dyn crate::session::storage::Storage>,
    ) -> Result<Self, PersistError> {
        let pending = storage.load_outbox()?;
        let next = pending.iter().map(|e| e.id.0).max().unwrap_or(0) + 1;
        Ok(Self {
            store,
            adapter,
            status: Arc::new(Mutex::new(SyncStatus {
                pending_events: pending.len(),
                ..SyncStatus::default()
            })),
            pending: Arc::new(Mutex::new(pending)),
            outbox_path: None,
            storage: Some(storage),
            codec: Arc::new(crate::board::adapters::gh_label_codec::GhLabelCodec),
            next_mutation_id: AtomicU64::new(next),
            action_tx: None,
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            ci_state_store: None,
            race_window_barrier: None,
        })
    }

    pub fn with_action_tx(
        mut self,
        tx: tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>,
    ) -> Self {
        self.action_tx = Some(tx);
        self
    }

    pub(super) fn persist_outbox(&self) {
        let snapshot: Vec<QueuedEvent> = self.with_pending(|p| p.iter().cloned().collect());
        // #1289 Phase-1 step 4: prefer Storage-trait sink when set
        // (with_storage construction); fall back to outbox_path for
        // backward-compat with `with_outbox_path`-constructed instances.
        if let Some(storage) = self.storage.as_ref() {
            if let Err(e) = storage.save_outbox(&snapshot) {
                crate::session::diag::log_diag(&format!(
                    "orchestrator: outbox persist via storage failed: {e}"
                ));
            }
            return;
        }
        let Some(path) = self.outbox_path.as_ref() else {
            return;
        };
        if let Err(e) = persist_outbox_to_disk(path, &snapshot) {
            crate::session::diag::log_diag(&format!("orchestrator: outbox persist failed: {e}"));
        }
    }

    pub fn status_handle(&self) -> Arc<Mutex<SyncStatus>> {
        Arc::clone(&self.status)
    }

    /// Transactional-closure read over the pending-events deque. The
    /// std::sync::Mutex is locked once at entry (with poison recovery)
    /// and dropped at closure-return, so the lock-scope is visibly
    /// bounded at the call-site instead of spanning the enclosing fn
    /// body.
    pub(super) fn with_pending<R>(&self, f: impl FnOnce(&VecDeque<QueuedEvent>) -> R) -> R {
        let p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        f(&p)
    }

    /// Transactional-closure mutation over the pending-events deque.
    /// Same lock-scope-bounded contract as `with_pending` for read-write
    /// access. Per Ryan-canon "prefer immutability" (#1360): mutation
    /// scope is bounded + visible at the call-site, not scattered.
    pub(super) fn with_pending_mut<R>(&self, f: impl FnOnce(&mut VecDeque<QueuedEvent>) -> R) -> R {
        let mut p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut p)
    }

    #[cfg(test)]
    pub fn snapshot_pending_for_test(&self) -> VecDeque<QueuedEvent> {
        self.with_pending(|p| p.clone())
    }

    pub fn store(&self) -> Arc<BoardStore> {
        Arc::clone(&self.store)
    }

    pub async fn ingest_once(&self) -> Result<(), SyncError> {
        self.ingest_once_since(None).await
    }

    pub async fn ingest_once_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), SyncError> {
        match self.adapter.ingest_metadata_since(since).await {
            Ok(stories) => {
                self.store.seed_all(stories);
                let purge = super::adapters::github::iteration_numbers_to_purge_as_retired_empty(
                    &self.store.all_stories(),
                );
                for n in purge {
                    self.store.remove(n);
                }
                record_ingest_success(&self.status);
                Ok(())
            }
            Err(e) => {
                record_error(&self.status, &format!("ingest: {e}"));
                Err(e)
            }
        }
    }

    pub async fn refresh_story(
        &self,
        number: u64,
    ) -> Result<Option<super::domain::Story>, SyncError> {
        match self.adapter.fetch_story_metadata(number).await {
            Ok(Some(story)) => {
                self.store.seed_all(vec![story.clone()]);
                record_ingest_success(&self.status);
                Ok(Some(story))
            }
            Ok(None) => {
                record_ingest_success(&self.status);
                Ok(None)
            }
            Err(e) => {
                record_error(&self.status, &format!("refresh_story({number}): {e}"));
                Err(e)
            }
        }
    }

    pub async fn refresh_all(&self) -> Result<(), SyncError> {
        self.ingest_once().await
    }

    pub async fn fetch_body(&self, number: u64) -> Result<String, SyncError> {
        match self.adapter.fetch_story_body(number).await {
            Ok(body) => {
                let event = BoardEvent::BodyFetched {
                    meta: cache_fill_meta(),
                    number,
                    body: body.clone(),
                    etag: None,
                };
                self.store.apply(&event);
                record_ingest_success(&self.status);
                Ok(body)
            }
            Err(e) => {
                record_error(&self.status, &format!("fetch_body({number}): {e}"));
                Err(e)
            }
        }
    }

    pub async fn fetch_body_conditional(
        &self,
        number: u64,
        prior_etag: Option<String>,
    ) -> Result<ConditionalBody, SyncError> {
        match self
            .adapter
            .fetch_story_body_conditional(number, prior_etag)
            .await
        {
            Ok(ConditionalBody::NotModified) => {
                record_ingest_success(&self.status);
                Ok(ConditionalBody::NotModified)
            }
            Ok(ConditionalBody::Modified { body, etag }) => {
                let event = BoardEvent::BodyFetched {
                    meta: cache_fill_meta(),
                    number,
                    body: body.clone(),
                    etag: etag.clone(),
                };
                self.store.apply(&event);
                record_ingest_success(&self.status);
                Ok(ConditionalBody::Modified { body, etag })
            }
            Err(e) => {
                record_error(
                    &self.status,
                    &format!("fetch_body_conditional({number}): {e}"),
                );
                Err(e)
            }
        }
    }

    pub async fn fetch_free_comments(&self, number: u64) -> Result<Vec<FreeComment>, SyncError> {
        match self.adapter.fetch_free_comments(number).await {
            Ok(comments) => {
                let event = BoardEvent::FreeCommentsFetched {
                    meta: cache_fill_meta(),
                    number,
                    comments: comments.clone(),
                };
                self.store.apply(&event);
                record_ingest_success(&self.status);
                Ok(comments)
            }
            Err(e) => {
                record_error(&self.status, &format!("fetch_free_comments({number}): {e}"));
                Err(e)
            }
        }
    }

    pub async fn fetch_gate_comments(&self, number: u64) -> Result<Vec<GateComment>, SyncError> {
        match self.adapter.fetch_gate_comments(number).await {
            Ok(comments) => {
                let event = BoardEvent::GateCommentsFetched {
                    meta: cache_fill_meta(),
                    number,
                    comments: comments.clone(),
                };
                self.store.apply(&event);
                record_ingest_success(&self.status);
                Ok(comments)
            }
            Err(e) => {
                record_error(&self.status, &format!("fetch_gate_comments({number}): {e}"));
                Err(e)
            }
        }
    }

    pub fn apply_server_rollback(&self, story: Story) {
        self.store.seed_all(vec![story]);
    }

    pub fn pending_issue_for_mutation(&self, id: MutationId) -> Option<u64> {
        self.with_pending(|p| {
            p.iter()
                .find(|q| q.id == id)
                .map(|q| q.event.story_number())
        })
    }

    pub async fn issue_lock(&self, issue: u64) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.issue_locks.lock().await;
        Arc::clone(
            locks
                .entry(issue)
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    }

    pub fn snapshot_status(&self) -> SyncStatus {
        let snapshot = self
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        SyncStatus {
            stale: is_stale(
                snapshot.last_ingest_at,
                staleness_threshold_secs(),
                Utc::now(),
            ),
            startup_fail: is_startup_fail(snapshot.last_ingest_at, snapshot.last_error.as_deref()),
            ..snapshot
        }
    }

    pub async fn create_story(&self, title: &str, body: &str) -> Result<u64, SyncError> {
        match self.adapter.create_story(title, body).await {
            Ok(number) => {
                record_push_success(&self.status);
                Ok(number)
            }
            Err(e) => {
                record_error(&self.status, &format!("create_story: {e}"));
                Err(e)
            }
        }
    }

    pub async fn reconcile_dual_state_labels(&self) -> Result<usize, SyncError> {
        let stories = self.store.all_stories();
        let mut repairs = 0usize;
        for story in stories {
            let labels = match self.adapter.fetch_story_labels(story.number).await {
                Ok(l) => l,
                Err(_) => continue,
            };
            let stale = stale_state_labels_for(&story, &labels);
            for stale_label in stale {
                let meta = super::events::EventMeta {
                    actor: "bpmux:reconcile".to_string(),
                    ts: Utc::now(),
                    peer_id: super::peer::PeerId::local(),
                };
                self.mutate(BoardEvent::TagRemoved {
                    meta,
                    number: story.number,
                    label: stale_label,
                })?;
                repairs += 1;
            }
        }
        if repairs > 0 {
            self.drain_once().await?;
        }
        Ok(repairs)
    }
}

impl<A: SourceAdapter + 'static> RefreshDriver for SyncOrchestrator<A> {
    fn refresh_story(&self, number: u64) -> BoxedFuture<'_, Result<Option<Story>, SyncError>> {
        Box::pin(self.refresh_story(number))
    }

    fn refresh_all(&self) -> BoxedFuture<'_, Result<(), SyncError>> {
        Box::pin(self.refresh_all())
    }

    fn refresh_all_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> BoxedFuture<'_, Result<(), SyncError>> {
        Box::pin(self.ingest_once_since(since))
    }

    fn mutate_sync(&self, event: BoardEvent) -> Result<(), SyncError> {
        self.mutate(event)
    }

    fn drain_one_by_id(&self, id: MutationId) -> BoxedFuture<'_, DrainOneOutcome> {
        Box::pin(self.drain_one_by_id(id))
    }

    fn apply_server_rollback(&self, story: Story) -> BoxedFuture<'_, ()> {
        let store = Arc::clone(&self.store);
        Box::pin(async move {
            store.seed_all(vec![story]);
        })
    }

    fn snapshot_pending_for_mutation(&self, id: MutationId) -> Option<u64> {
        self.pending_issue_for_mutation(id)
    }

    fn issue_lock(&self, issue: u64) -> BoxedFuture<'_, Arc<tokio::sync::Mutex<()>>> {
        Box::pin(self.issue_lock(issue))
    }

    fn snapshot_status(&self) -> SyncStatus {
        self.snapshot_status()
    }

    fn acknowledge_dropped(&self) -> usize {
        acknowledge_dropped(&self.status)
    }

    fn mark_recovering_from_corruption(&self) {
        mark_recovering_from_corruption(&self.status);
    }

    fn drain_pending(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
        Box::pin(self.drain_once())
    }

    fn reconcile_dual_state_labels(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
        Box::pin(self.reconcile_dual_state_labels())
    }

    fn fetch_body(&self, number: u64) -> BoxedFuture<'_, Result<String, SyncError>> {
        Box::pin(self.fetch_body(number))
    }

    fn fetch_body_conditional(
        &self,
        number: u64,
        prior_etag: Option<String>,
    ) -> BoxedFuture<'_, Result<ConditionalBody, SyncError>> {
        Box::pin(self.fetch_body_conditional(number, prior_etag))
    }

    fn fetch_free_comments(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<FreeComment>, SyncError>> {
        Box::pin(self.fetch_free_comments(number))
    }

    fn fetch_gate_comments(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<GateComment>, SyncError>> {
        Box::pin(self.fetch_gate_comments(number))
    }

    fn create_story(&self, title: String, body: String) -> BoxedFuture<'_, Result<u64, SyncError>> {
        Box::pin(async move { self.create_story(&title, &body).await })
    }

    fn story(&self, number: u64) -> Option<Story> {
        self.store.story(number)
    }

    fn all_stories(&self) -> Vec<Story> {
        self.store.all_stories()
    }

    fn store(&self) -> Arc<BoardStore> {
        Arc::clone(&self.store)
    }

    fn update_body(&self, number: u64, body: String) -> BoxedFuture<'_, Result<(), SyncError>> {
        let adapter = Arc::clone(&self.adapter);
        let store = Arc::clone(&self.store);
        Box::pin(async move {
            // #1151: capture post-update Story from the adapter and
            // apply it to the canonical store so callers don't need
            // a follow-up `refresh_story(N)` to observe the new body.
            let story = adapter.update_body(number, &body).await?;
            store.apply(&BoardEvent::StoryRefreshed {
                meta: cache_fill_meta(),
                number,
                story: Box::new(story),
            });
            Ok(())
        })
    }

    fn update_title(&self, number: u64, title: String) -> BoxedFuture<'_, Result<(), SyncError>> {
        let adapter = Arc::clone(&self.adapter);
        let store = Arc::clone(&self.store);
        Box::pin(async move {
            let story = adapter.update_title(number, &title).await?;
            store.apply(&BoardEvent::StoryRefreshed {
                meta: cache_fill_meta(),
                number,
                story: Box::new(story),
            });
            Ok(())
        })
    }

    fn ci_state(&self) -> Option<crate::workflow::CiSnapshot> {
        let store = self.ci_state_store.as_ref()?;
        let report = store.read()?;
        Some(crate::workflow::CiSnapshot {
            conclusion: report.conclusion.to_string(),
            head_sha: report.head_sha.unwrap_or_default(),
        })
    }

    fn test_race_window_rendezvous(&self) -> BoxedFuture<'_, ()> {
        let barrier = self.race_window_barrier.clone();
        Box::pin(async move {
            if let Some(b) = barrier {
                b.arrive().await;
            }
        })
    }

    fn test_race_window_complete(&self) {
        if let Some(b) = self.race_window_barrier.as_ref() {
            b.complete();
        }
    }
}

pub(crate) fn cache_fill_meta() -> EventMeta {
    EventMeta {
        actor: "source-ingest".to_string(),
        ts: Utc::now(),
        peer_id: PeerId::local(),
    }
}

use outbox::persist_outbox_to_disk;
