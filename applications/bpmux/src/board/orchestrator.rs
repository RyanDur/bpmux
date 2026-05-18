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
    fn store(&self) -> Arc<BoardStore>;
    fn update_body(&self, number: u64, body: String) -> BoxedFuture<'_, Result<(), SyncError>>;

    fn update_title(&self, number: u64, title: String) -> BoxedFuture<'_, Result<(), SyncError>>;

    fn ci_state(&self) -> Option<crate::workflow::CiSnapshot> {
        None
    }

    fn test_race_window_rendezvous(&self) -> BoxedFuture<'_, ()> {
        Box::pin(async {})
    }

    fn test_race_window_complete(&self) {}
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    #[serde(default)]
    pub id: MutationId,
    pub event: BoardEvent,
    #[serde(default)]
    pub pre_state: Option<Story>,
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
    pub(super) storage: Option<Arc<dyn crate::session::storage::Storage>>,
    pub(super) codec: Arc<dyn crate::board::label_codec::LabelCodec + Send + Sync>,
    pub(super) next_mutation_id: AtomicU64,
    pub(super) action_tx:
        Option<tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>>,
    pub(super) issue_locks:
        tokio::sync::Mutex<std::collections::HashMap<u64, Arc<tokio::sync::Mutex<()>>>>,
    pub(super) ci_state_store: Option<crate::transport::ci_state_store::CiStateStore>,
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

    pub fn with_ci_state_store(
        mut self,
        store: crate::transport::ci_state_store::CiStateStore,
    ) -> Self {
        self.ci_state_store = Some(store);
        self
    }

    pub fn with_race_window_barrier(mut self, barrier: Arc<RaceWindowBarrier>) -> Self {
        self.race_window_barrier = Some(barrier);
        self
    }

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

    pub(super) fn with_pending<R>(&self, f: impl FnOnce(&VecDeque<QueuedEvent>) -> R) -> R {
        let p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        f(&p)
    }

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
