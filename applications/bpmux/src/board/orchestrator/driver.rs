use std::sync::Arc;

use crate::board::domain::{FreeComment, GateComment, Story};
use crate::board::events::BoardEvent;
use crate::board::store::BoardStore;
use crate::board::sync::{ConditionalBody, SyncError};

use super::{BoxedFuture, DrainOneOutcome, MutationId, SyncStatus};

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
    fn status_handle(&self) -> Arc<std::sync::Mutex<SyncStatus>>;
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
    fn create_story(
        &self,
        title: String,
        body: String,
        initial_labels: Vec<String>,
    ) -> BoxedFuture<'_, Result<u64, SyncError>>;
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
