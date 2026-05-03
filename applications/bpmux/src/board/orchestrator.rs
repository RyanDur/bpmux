use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::mpsc;

use super::domain::{FreeComment, GateComment, Story};
use super::events::{BoardEvent, EventMeta};
use super::peer::PeerId;
use super::store::BoardStore;
use super::sync::{SourceAdapter, SyncError};

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait RefreshDriver: Send + Sync {
    fn refresh_story(&self, number: u64) -> BoxedFuture<'_, Result<Option<Story>, SyncError>>;
    fn refresh_all(&self) -> BoxedFuture<'_, Result<(), SyncError>>;
    fn mutate_sync(&self, event: BoardEvent) -> Result<(), SyncError>;
    fn snapshot_status(&self) -> SyncStatus;
    fn drain_pending(&self) -> BoxedFuture<'_, Result<usize, SyncError>>;
    fn reconcile_dual_state_labels(&self) -> BoxedFuture<'_, Result<usize, SyncError>>;
    fn fetch_body(&self, number: u64) -> BoxedFuture<'_, Result<String, SyncError>>;
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
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncStatus {
    pub last_ingest_at: Option<DateTime<Utc>>,
    pub last_push_at: Option<DateTime<Utc>>,
    pub pending_events: usize,
    pub last_error: Option<String>,
}

pub struct SyncOrchestrator<A: SourceAdapter + 'static> {
    store: Arc<BoardStore>,
    adapter: Arc<A>,
    status: Arc<Mutex<SyncStatus>>,
    pending_tx: mpsc::UnboundedSender<BoardEvent>,
    pending_rx: Mutex<Option<mpsc::UnboundedReceiver<BoardEvent>>>,
}

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn new(store: Arc<BoardStore>, adapter: Arc<A>) -> Self {
        let (pending_tx, pending_rx) = mpsc::unbounded_channel();
        Self {
            store,
            adapter,
            status: Arc::new(Mutex::new(SyncStatus::default())),
            pending_tx,
            pending_rx: Mutex::new(Some(pending_rx)),
        }
    }

    pub fn status_handle(&self) -> Arc<Mutex<SyncStatus>> {
        Arc::clone(&self.status)
    }

    pub fn store(&self) -> Arc<BoardStore> {
        Arc::clone(&self.store)
    }

    pub fn mutate(&self, event: BoardEvent) -> Result<(), SyncError> {
        self.store.apply(&event);
        self.pending_tx
            .send(event)
            .map_err(|e| SyncError::Other(format!("pending-buffer closed: {e}")))?;
        bump_pending(&self.status, 1);
        Ok(())
    }

    pub async fn ingest_once(&self) -> Result<(), SyncError> {
        match self.adapter.ingest_metadata().await {
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

    pub async fn drain_once(&self) -> Result<usize, SyncError> {
        let mut rx = self
            .pending_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .ok_or_else(|| SyncError::Other("pending-receiver already taken".into()))?;
        let mut drained = 0usize;
        let mut failures = 0usize;
        while let Ok(event) = rx.try_recv() {
            match self.adapter.push_event(&event).await {
                Ok(follow_up) => {
                    drained += 1;
                    record_push_success(&self.status);
                    // #1151: the adapter captured GH's mutation
                    // response and surfaced it as a follow-up
                    // BoardEvent (StoryRefreshed for Issue-returning
                    // mutations; LabelsRefreshed for label-mutation
                    // responses). Dispatch via the reducer so the
                    // local store reflects post-mutation state
                    // without a follow-up `refresh_story(N)` call.
                    if let Some(refresh_event) = follow_up {
                        self.store.apply(&refresh_event);
                    }
                }
                Err(e) => {
                    failures += 1;
                    record_error(&self.status, &format!("push: {e}"));
                    let _ = self.pending_tx.send(event);
                    break;
                }
            }
        }
        *self.pending_rx.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);
        if failures > 0 {
            return Err(SyncError::Other(format!(
                "{failures} push failures; {drained} pushes succeeded"
            )));
        }
        Ok(drained)
    }

    pub fn snapshot_status(&self) -> SyncStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
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

    fn mutate_sync(&self, event: BoardEvent) -> Result<(), SyncError> {
        self.mutate(event)
    }

    fn snapshot_status(&self) -> SyncStatus {
        self.snapshot_status()
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
                story,
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

fn record_ingest_success(status: &Arc<Mutex<SyncStatus>>) {
    let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
    s.last_ingest_at = Some(Utc::now());
    s.last_error = None;
}

fn record_push_success(status: &Arc<Mutex<SyncStatus>>) {
    let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
    s.last_push_at = Some(Utc::now());
    if s.pending_events > 0 {
        s.pending_events -= 1;
    }
}

fn record_error(status: &Arc<Mutex<SyncStatus>>, msg: &str) {
    let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
    s.last_error = Some(msg.to_string());
}

fn bump_pending(status: &Arc<Mutex<SyncStatus>>, by: usize) {
    let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
    s.pending_events += by;
}
