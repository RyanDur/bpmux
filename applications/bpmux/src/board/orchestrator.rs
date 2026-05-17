use std::collections::VecDeque;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::domain::{FreeComment, GateComment, Story};
use super::events::{BoardEvent, EventMeta};
use super::peer::PeerId;
use super::store::{BoardStore, PersistError};
use super::sync::{ConditionalBody, SourceAdapter, SyncError, is_permanent_push_failure};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MutationId(pub u64);

impl std::fmt::Display for MutationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn default_outbox_path() -> Result<PathBuf, std::io::Error> {
    if let Ok(p) = std::env::var("BPMUX_BOARD_OUTBOX_PATH")
        && !p.is_empty()
    {
        return Ok(PathBuf::from(p));
    }
    let base = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(base.join(".bpmux").join("board-outbox.json"))
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
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncStatus {
    pub last_ingest_at: Option<DateTime<Utc>>,
    pub last_push_at: Option<DateTime<Utc>>,
    pub pending_events: usize,
    pub last_error: Option<String>,
    pub dropped_events: Vec<DroppedEvent>,
    pub recovering_from_corruption: bool,
    #[serde(default)]
    pub stale: bool,
    #[serde(default)]
    pub startup_fail: bool,
    /// #1195: per-op count of currently-conflicted outbox entries.
    /// Each drain tick re-classifies; a conflicted entry (server
    /// state diverged externally between mutate-time and drain-time)
    /// stays in the outbox + bumps this count for SyncStatusGlyph
    /// state #5 (slice B) to render. Idempotent skips do NOT count
    /// (they get dropped + diag-logged). Auto-resolved on next
    /// drain tick when the divergent server state catches up.
    #[serde(default)]
    pub conflicts_pending: usize,
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

fn iteration_prefix_class(label: &str) -> Option<String> {
    use crate::board::label_codec::LabelCodec;
    crate::board::adapters::gh_label_codec::GhLabelCodec.iteration_label_class(label)
}

fn coalesce_iteration_label_events(
    queue: &mut VecDeque<QueuedEvent>,
    new_event: &BoardEvent,
    new_id: MutationId,
    new_pre_state: &Option<Story>,
) -> bool {
    let (number, label, is_add) = match new_event {
        BoardEvent::TagAdded { number, label, .. } => (*number, label.clone(), true),
        BoardEvent::TagRemoved { number, label, .. } => (*number, label.clone(), false),
        _ => return false,
    };
    let Some(prefix) = iteration_prefix_class(&label) else {
        return false;
    };
    let class_indices: Vec<usize> = queue
        .iter()
        .enumerate()
        .filter_map(|(i, q)| match &q.event {
            BoardEvent::TagAdded {
                number: n,
                label: l,
                ..
            }
            | BoardEvent::TagRemoved {
                number: n,
                label: l,
                ..
            } if *n == number && iteration_prefix_class(l).as_deref() == Some(prefix.as_str()) => {
                Some(i)
            }
            _ => None,
        })
        .collect();
    if class_indices.is_empty() {
        return false;
    }
    let mut first_r_idx: Option<usize> = None;
    let mut last_a_idx: Option<usize> = None;
    for &i in &class_indices {
        match &queue[i].event {
            BoardEvent::TagRemoved { .. } if first_r_idx.is_none() => first_r_idx = Some(i),
            BoardEvent::TagAdded { .. } => last_a_idx = Some(i),
            _ => {}
        }
    }
    let r_to_keep: Option<QueuedEvent>;
    let a_to_keep: Option<QueuedEvent>;
    if is_add {
        r_to_keep = first_r_idx.map(|i| queue[i].clone());
        a_to_keep = Some(QueuedEvent::new_with_id(
            new_id,
            new_event.clone(),
            new_pre_state.clone(),
        ));
    } else {
        r_to_keep = match first_r_idx {
            Some(i) => Some(queue[i].clone()),
            None => Some(QueuedEvent::new_with_id(
                new_id,
                new_event.clone(),
                new_pre_state.clone(),
            )),
        };
        a_to_keep = last_a_idx.map(|i| queue[i].clone());
    }
    let mut sorted = class_indices;
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    for i in sorted {
        queue.remove(i);
    }
    if let Some(r) = r_to_keep {
        queue.push_back(r);
    }
    if let Some(a) = a_to_keep {
        queue.push_back(a);
    }
    true
}

/// #1195 conflict classification result. The classifier compares
/// `pre_state` (snapshot captured at mutate-time) → `event` (the
/// intended mutation) → `current` (canonical-store-state at
/// drain-time, NOT GH state per #1194's no-per-op-GH-GET
/// discipline). Five outcomes:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictKind {
    /// The mutation is already applied at the canonical store and
    /// would be a no-op (label-already-present, already-closed,
    /// parent-epic-already-pointing-here). Drop without firing GH.
    IdempotentSkip,
    /// Server state diverged from pre_state in a way that conflicts
    /// with the event's intended state-transition (e.g., offline
    /// `StateTransitioned → Ready` was queued when server was
    /// `Delivered`, but server is now at `Accepted`). Operator
    /// click-to-resolve via slice B.
    DivergentState,
    /// Server label set diverged from pre_state in a way that
    /// conflicts with the event's intended label change. Operator
    /// click-to-resolve via slice B.
    DivergentLabel,
    /// External mutation arrived between queue-time and drain-time
    /// and the server-shifted state is now incompatible with the
    /// queued event's basic preconditions (e.g., we queued
    /// `TagAdded` against an open issue but server has since closed
    /// it). Sister to Divergent* but generalized.
    External,
}

pub fn staleness_threshold_secs() -> u64 {
    std::env::var("BPMUX_STALENESS_THRESHOLD_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(180)
}

pub fn is_stale(
    last_ingest_at: Option<DateTime<Utc>>,
    threshold_secs: u64,
    now: DateTime<Utc>,
) -> bool {
    let Some(last) = last_ingest_at else {
        return false;
    };
    let elapsed = now.signed_duration_since(last);
    elapsed.num_seconds() >= 0 && (elapsed.num_seconds() as u64) > threshold_secs
}

pub fn is_startup_fail(last_ingest_at: Option<DateTime<Utc>>, last_error: Option<&str>) -> bool {
    last_ingest_at.is_none() && last_error.is_some()
}

#[derive(Debug, Clone, Serialize)]
pub struct DroppedEvent {
    pub story_number: u64,
    pub error: String,
    pub ts: DateTime<Utc>,
}

pub struct SyncOrchestrator<A: SourceAdapter + 'static> {
    store: Arc<BoardStore>,
    adapter: Arc<A>,
    status: Arc<Mutex<SyncStatus>>,
    pending: Arc<Mutex<VecDeque<QueuedEvent>>>,
    outbox_path: Option<PathBuf>,
    /// #1289 Phase-1 step 4: Storage-trait sink for outbox persistence.
    /// When `Some`, used in preference to `outbox_path`. Set via
    /// `with_storage` constructor (sister of `with_outbox_path`).
    storage: Option<Arc<dyn crate::session::storage::Storage>>,
    next_mutation_id: AtomicU64,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>>,
    issue_locks: tokio::sync::Mutex<std::collections::HashMap<u64, Arc<tokio::sync::Mutex<()>>>>,
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
            next_mutation_id: AtomicU64::new(1),
            action_tx: None,
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
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
            next_mutation_id: AtomicU64::new(next),
            action_tx: None,
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
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
            next_mutation_id: AtomicU64::new(next),
            action_tx: None,
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        })
    }

    pub fn with_action_tx(
        mut self,
        tx: tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>,
    ) -> Self {
        self.action_tx = Some(tx);
        self
    }

    fn persist_outbox(&self) {
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
    fn with_pending<R>(&self, f: impl FnOnce(&VecDeque<QueuedEvent>) -> R) -> R {
        let p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        f(&p)
    }

    /// Transactional-closure mutation over the pending-events deque.
    /// Same lock-scope-bounded contract as `with_pending` for read-write
    /// access. Per Ryan-canon "prefer immutability" (#1360): mutation
    /// scope is bounded + visible at the call-site, not scattered.
    fn with_pending_mut<R>(&self, f: impl FnOnce(&mut VecDeque<QueuedEvent>) -> R) -> R {
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

    pub fn mutate(&self, event: BoardEvent) -> Result<(), SyncError> {
        // #1195: capture pre_state BEFORE applying the event so
        // drain-time can detect external mutations that arrived
        // between mutate-time and drain-time. Per
        // `feedback_mutate_sync_breaks_drain_time_idempotent_check`:
        // comparing event-vs-current-store at drain-time always
        // says "already applied" because the local apply ran first.
        // The pre_state captures temporal context the comparison
        // needs.
        let id = MutationId(self.next_mutation_id.fetch_add(1, Ordering::SeqCst));
        let pre_state = self.store.story(event.story_number());
        self.store.apply(&event);
        let queue_len = self.with_pending_mut(|p| {
            let absorbed = coalesce_iteration_label_events(p, &event, id, &pre_state);
            if !absorbed {
                p.push_back(QueuedEvent::new_with_id(id, event, pre_state));
            }
            p.len()
        });
        self.persist_outbox();
        set_pending_events(&self.status, queue_len);
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(crate::orchestrator::OrchestratorAction::StartUpdateGithub {
                mutation_id: id,
            });
        }
        Ok(())
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

    pub async fn drain_one_by_id(&self, id: MutationId) -> DrainOneOutcome {
        let queued = self.with_pending(|p| p.iter().find(|q| q.id == id).cloned());
        let Some(queued) = queued else {
            return DrainOneOutcome::NotFound;
        };
        let event = queued.event.clone();
        let lock = self.issue_lock(event.story_number()).await;
        let _guard = lock.lock().await;
        let queued = self.with_pending(|p| p.iter().find(|q| q.id == id).cloned());
        let Some(queued) = queued else {
            return DrainOneOutcome::Confirmed;
        };
        let event = queued.event.clone();
        let current = self.store.story(event.story_number());
        let kind = classify_conflict(queued.pre_state.as_ref(), &event, current.as_ref());
        match kind {
            Some(ConflictKind::IdempotentSkip) => {
                self.remove_pending_by_id(id);
                self.persist_outbox();
                decrement_pending(&self.status);
                crate::session::diag::log_diag(&format!(
                    "orchestrator: idempotent-skip dropped #{} via drain_one_by_id (already-applied)",
                    event.story_number()
                ));
                DrainOneOutcome::Confirmed
            }
            Some(divergent) => {
                self.remove_pending_by_id(id);
                self.persist_outbox();
                decrement_pending(&self.status);
                self.store.apply(&BoardEvent::ConflictDetected {
                    meta: cache_fill_meta(),
                    number: event.story_number(),
                    kind: divergent,
                });
                crate::session::diag::log_diag(&format!(
                    "orchestrator: drain_one_by_id rolled back #{} ({divergent:?}); applying server_story",
                    event.story_number()
                ));
                DrainOneOutcome::RolledBack {
                    server_story: current.map(Box::new),
                }
            }
            None => match self.adapter.push_event(&event).await {
                Ok(follow_up) => {
                    self.remove_pending_by_id(id);
                    self.persist_outbox();
                    decrement_pending(&self.status);
                    record_push_success(&self.status);
                    if let Some(refresh_event) = follow_up {
                        self.store.apply(&refresh_event);
                    }
                    DrainOneOutcome::Confirmed
                }
                Err(e) => {
                    record_error(&self.status, &format!("push: {e}"));
                    if is_permanent_push_failure(&e) {
                        self.remove_pending_by_id(id);
                        self.persist_outbox();
                        decrement_pending(&self.status);
                        record_dropped(&self.status, event.story_number(), format!("{e}"));
                        crate::session::diag::log_diag(&format!(
                            "orchestrator: drain_one_by_id dropped #{} after permanent push failure ({e})",
                            event.story_number()
                        ));
                        DrainOneOutcome::Confirmed
                    } else {
                        DrainOneOutcome::TransientError(e)
                    }
                }
            },
        }
    }

    fn remove_pending_by_id(&self, id: MutationId) {
        self.with_pending_mut(|p| {
            if let Some(idx) = p.iter().position(|q| q.id == id) {
                p.remove(idx);
            }
        });
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

    pub async fn drain_once(&self) -> Result<usize, SyncError> {
        let mut drained = 0usize;
        let mut failures = 0usize;
        // #1195 conflict-detection bookkeeping. We walk the outbox
        // by index rather than always pop-from-front so a divergent
        // entry can stay in place while non-conflicting entries
        // behind it continue to drain (per #1163 falsifier-c on
        // bulk-batch partial-conflict semantics).
        let mut conflicts_pending = 0usize;
        let mut conflict_offset = 0usize;
        loop {
            let queued_opt = self.with_pending(|p| p.get(conflict_offset).cloned());
            let Some(queued) = queued_opt else {
                break;
            };
            let event = queued.event.clone();
            let lock = self.issue_lock(event.story_number()).await;
            let _guard = lock.lock().await;
            let queued_opt = self.with_pending(|p| p.get(conflict_offset).cloned());
            let Some(queued) = queued_opt else {
                break;
            };
            let event = queued.event.clone();
            let current = self.store.story(event.story_number());
            let prior_classification = queued.last_classification;
            let kind = classify_conflict(queued.pre_state.as_ref(), &event, current.as_ref());

            // Auto-resolve: a previously-conflicted entry that now
            // classifies as compatible (None) means the server has
            // caught up via ingest poll. Emit ConflictResolved so
            // the audit-trail records the auto-clear.
            if matches!(
                prior_classification,
                Some(
                    ConflictKind::DivergentState
                        | ConflictKind::DivergentLabel
                        | ConflictKind::External
                )
            ) && kind.is_none()
            {
                self.store.apply(&BoardEvent::ConflictResolved {
                    meta: cache_fill_meta(),
                    number: event.story_number(),
                    origin: ConflictResolveOrigin::DrainAutoReclassify,
                });
                crate::session::diag::log_diag(&format!(
                    "orchestrator: conflict auto-resolved on drain re-classify for #{}",
                    event.story_number()
                ));
            }

            match kind {
                Some(ConflictKind::IdempotentSkip) => {
                    self.with_pending_mut(|p| {
                        if conflict_offset == 0 {
                            p.pop_front();
                        } else {
                            p.remove(conflict_offset);
                        }
                    });
                    self.persist_outbox();
                    decrement_pending(&self.status);
                    crate::session::diag::log_diag(&format!(
                        "orchestrator: idempotent-skip dropped #{} (already-applied at canonical store)",
                        event.story_number()
                    ));
                    continue;
                }
                Some(divergent) => {
                    if !matches!(
                        prior_classification,
                        Some(
                            ConflictKind::DivergentState
                                | ConflictKind::DivergentLabel
                                | ConflictKind::External
                        )
                    ) {
                        // First detection of this conflict — emit
                        // the audit-trail event so observers know
                        // why this entry stays in the outbox.
                        self.store.apply(&BoardEvent::ConflictDetected {
                            meta: cache_fill_meta(),
                            number: event.story_number(),
                            kind: divergent,
                        });
                        crate::session::diag::log_diag(&format!(
                            "orchestrator: conflict-detected {divergent:?} on #{}; left in outbox for slice-B click-to-resolve",
                            event.story_number()
                        ));
                    }
                    self.with_pending_mut(|p| {
                        if let Some(slot) = p.get_mut(conflict_offset) {
                            slot.last_classification = Some(divergent);
                        }
                    });
                    self.persist_outbox();
                    conflicts_pending += 1;
                    conflict_offset += 1;
                    continue;
                }
                None => {}
            }
            match self.adapter.push_event(&event).await {
                Ok(follow_up) => {
                    self.with_pending_mut(|p| {
                        if conflict_offset == 0 {
                            p.pop_front();
                        } else {
                            p.remove(conflict_offset);
                        }
                    });
                    self.persist_outbox();
                    drained += 1;
                    record_push_success(&self.status);
                    if let Some(refresh_event) = follow_up {
                        self.store.apply(&refresh_event);
                    }
                }
                Err(e) => {
                    failures += 1;
                    record_error(&self.status, &format!("push: {e}"));
                    if is_permanent_push_failure(&e) {
                        self.with_pending_mut(|p| {
                            if conflict_offset == 0 {
                                p.pop_front();
                            } else {
                                p.remove(conflict_offset);
                            }
                        });
                        self.persist_outbox();
                        decrement_pending(&self.status);
                        record_dropped(&self.status, event.story_number(), format!("{e}"));
                        crate::session::diag::log_diag(&format!(
                            "orchestrator: dropped event after permanent push failure ({e}); event #{}",
                            event.story_number()
                        ));
                        continue;
                    }
                    break;
                }
            }
        }
        record_conflicts_pending(&self.status, conflicts_pending);
        if failures > 0 {
            return Err(SyncError::Other(format!(
                "{failures} push failures; {drained} pushes succeeded"
            )));
        }
        Ok(drained)
    }

    pub fn snapshot_status(&self) -> SyncStatus {
        let mut status = self
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        status.stale = is_stale(
            status.last_ingest_at,
            staleness_threshold_secs(),
            Utc::now(),
        );
        status.startup_fail = is_startup_fail(status.last_ingest_at, status.last_error.as_deref());
        status
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

fn stale_state_labels_for(story: &Story, raw_labels: &[String]) -> Vec<String> {
    let expected = match story.state {
        super::domain::StoryState::Icebox => "",
        super::domain::StoryState::Ready => "ready",
        super::domain::StoryState::InProgress => "in progress",
        super::domain::StoryState::DeskCheck => "desk-check",
        super::domain::StoryState::Delivered => "delivered",
        super::domain::StoryState::Accepted => "accepted",
    };
    let all_state_labels = [
        "ready",
        "in progress",
        "desk-check",
        "delivered",
        "ready for accept",
        "needs fixes",
        "rejected",
        "accepted",
    ];
    raw_labels
        .iter()
        .filter(|l| {
            let lower = l.to_ascii_lowercase();
            all_state_labels.contains(&lower.as_str()) && lower != expected
        })
        .cloned()
        .collect()
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
}

pub(crate) fn cache_fill_meta() -> EventMeta {
    EventMeta {
        actor: "source-ingest".to_string(),
        ts: Utc::now(),
        peer_id: PeerId::local(),
    }
}

/// Transactional-closure mutation over a `SyncStatus` behind an
/// `Arc<Mutex<_>>`. Free-fn form so the standalone `record_*` /
/// `mark_*` helpers below can route their lock-acquire through one
/// canonical site instead of each holding its own. Per #1360: lock-
/// scope bounded + visible at the call-site, not scattered.
fn with_status_mut<R>(status: &Arc<Mutex<SyncStatus>>, f: impl FnOnce(&mut SyncStatus) -> R) -> R {
    let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut s)
}

fn record_ingest_success(status: &Arc<Mutex<SyncStatus>>) {
    with_status_mut(status, |s| {
        s.last_ingest_at = Some(Utc::now());
        s.last_error = None;
        s.recovering_from_corruption = false;
    });
}

pub fn mark_recovering_from_corruption(status: &Arc<Mutex<SyncStatus>>) {
    with_status_mut(status, |s| {
        s.recovering_from_corruption = true;
    });
}

fn record_push_success(status: &Arc<Mutex<SyncStatus>>) {
    with_status_mut(status, |s| {
        s.last_push_at = Some(Utc::now());
        if s.pending_events > 0 {
            s.pending_events -= 1;
        }
    });
}

fn record_error(status: &Arc<Mutex<SyncStatus>>, msg: &str) {
    with_status_mut(status, |s| {
        s.last_error = Some(msg.to_string());
    });
}

fn set_pending_events(status: &Arc<Mutex<SyncStatus>>, value: usize) {
    with_status_mut(status, |s| {
        s.pending_events = value;
    });
}

fn decrement_pending(status: &Arc<Mutex<SyncStatus>>) {
    with_status_mut(status, |s| {
        if s.pending_events > 0 {
            s.pending_events -= 1;
        }
    });
}

fn record_dropped(status: &Arc<Mutex<SyncStatus>>, story_number: u64, error: String) {
    with_status_mut(status, |s| {
        s.dropped_events.push(DroppedEvent {
            story_number,
            error,
            ts: Utc::now(),
        });
    });
}

/// #1195: write the count of currently-conflicted outbox entries
/// into SyncStatus. Each drain tick re-runs the classifier on
/// every pending entry, so this fully reflects the current outbox
/// state — no incremental accumulation across ticks.
fn record_conflicts_pending(status: &Arc<Mutex<SyncStatus>>, count: usize) {
    with_status_mut(status, |s| {
        s.conflicts_pending = count;
    });
}

/// #1195: 3-arg conflict classifier. Compares pre-mutation
/// snapshot (captured at queue-time) against the event's intended
/// mutation against the current canonical-store state. Pure
/// function — no I/O. Returns `None` when the event is fine to
/// push as-is.
///
/// Per #1194's no-per-op-GH-GET shipped substrate, `current` is
/// the canonical-store-state, NOT a fresh GH fetch. The ingest
/// poller (60s default) is what brings server-truth into the
/// canonical store; this function trusts that view.
///
/// `pre_state: None` means: legacy outbox entry from before #1195
/// OR the story didn't exist in the store at queue-time. The
/// classifier degrades to no-conflict-detection (returns None) in
/// that case — we can't tell if the world changed without the
/// snapshot, so we err on "push and let the adapter surface any
/// error".
pub fn classify_conflict(
    _pre_state: Option<&Story>,
    event: &BoardEvent,
    _current: Option<&Story>,
) -> Option<ConflictKind> {
    // Slice A ships SUBSTRATE only: QueuedEvent wrapper + pre_state
    // capture + ConflictDetected/Resolved variants + SyncStatus
    // .conflicts_pending field + outbox schema migration. The
    // classifier returns None for now because correct semantics
    // require tracking cumulative-local-mutation-effects separately
    // from GH-fetched state (raw_labels reflects fetch-state, not
    // local-applied state until LabelsRefreshed roundtrips). That
    // scope-narrows the comparison to "intended-vs-server-truth"
    // which is slice B+ work. The substrate IS in place so slice B
    // plugs in the real classifier without further migrations.
    match event {
        BoardEvent::TagAdded { .. }
        | BoardEvent::TagRemoved { .. }
        | BoardEvent::StateTransitioned { .. }
        | BoardEvent::StoryClosed { .. }
        | BoardEvent::StoryReopened { .. }
        | BoardEvent::ParentEpicSet { .. }
        | BoardEvent::ParentEpicUnset { .. } => None,
        // Annotation/observation events are additive and not
        // conflict-classifiable in this slice.
        BoardEvent::StoryCreated { .. }
        | BoardEvent::GateCommentAppended { .. }
        | BoardEvent::FreeCommentAppended { .. }
        | BoardEvent::BodyFetched { .. }
        | BoardEvent::FreeCommentsFetched { .. }
        | BoardEvent::GateCommentsFetched { .. }
        | BoardEvent::StoryRefreshed { .. }
        | BoardEvent::LabelsRefreshed { .. }
        | BoardEvent::ConflictDetected { .. }
        | BoardEvent::ConflictResolved { .. } => None,
    }
}

/// #1195: origin tag for ConflictResolved events. Slice A only
/// fires the auto-reclassify origin (drain detects compatibility);
/// slice B adds OperatorAction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolveOrigin {
    DrainAutoReclassify,
    OperatorAction,
}

pub fn acknowledge_dropped(status: &Arc<Mutex<SyncStatus>>) -> usize {
    with_status_mut(status, |s| {
        let cleared = s.dropped_events.len();
        s.dropped_events.clear();
        cleared
    })
}

/// #1195: outbox-entry on-disk shape. Untagged enum lets us read
/// both the new wrapped form (`{"event":{...},"pre_state":...}`)
/// and the legacy bare-BoardEvent form (`{"StoryCreated":{...}}`)
/// from the same file. New writes always use the wrapped form;
/// legacy entries get pre_state: None on read (degrade to
/// no-conflict-detection — the classifier returns None when
/// pre_state is missing). Both variants boxed so the enum stays
/// pointer-sized (clippy::large_enum_variant — `Story` inside
/// QueuedEvent makes the variant fat).
#[derive(Deserialize)]
#[serde(untagged)]
enum OutboxEntryOnDisk {
    NewShape(Box<QueuedEvent>),
    LegacyShape(Box<BoardEvent>),
}

impl From<OutboxEntryOnDisk> for QueuedEvent {
    fn from(disk: OutboxEntryOnDisk) -> QueuedEvent {
        match disk {
            OutboxEntryOnDisk::NewShape(q) => *q,
            OutboxEntryOnDisk::LegacyShape(event) => QueuedEvent::from_legacy(*event),
        }
    }
}

pub fn load_outbox_from_disk(path: &Path) -> Result<VecDeque<QueuedEvent>, PersistError> {
    if !path.exists() {
        return Ok(VecDeque::new());
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Ok(VecDeque::new());
    }
    let entries: Vec<OutboxEntryOnDisk> = serde_json::from_slice(&bytes)?;
    Ok(entries.into_iter().map(QueuedEvent::from).collect())
}

fn persist_outbox_to_disk(path: &Path, events: &[QueuedEvent]) -> Result<(), PersistError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(events)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
