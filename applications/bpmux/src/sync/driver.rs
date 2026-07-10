use std::sync::Arc;

use crate::product_management::epic::{Epic, EpicId};
use crate::product_management::evidence::Evidence;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::outcome::Outcome;
use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{CastVerdict, FreeComment, Stage, Story};
use crate::sync::{ConditionalBody, SyncError};

use super::{BoxedFuture, SyncStatus};

pub type OrchestratorHandle = Option<Arc<dyn RefreshDriver>>;

pub trait RefreshDriver: Send + Sync {
    fn bind_select(&self, _select: crate::store::Select) {}
    fn bind_dispatch(&self, _dispatch: crate::store::Dispatch) {}
    fn refresh_story(&self, id: u64) -> BoxedFuture<'_, Result<Option<Story>, SyncError>>;
    fn refresh_all(&self) -> BoxedFuture<'_, Result<(), SyncError>>;
    fn refresh_all_since(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> BoxedFuture<'_, Result<(), SyncError>> {
        self.refresh_all()
    }
    fn mutate_sync(&self, event: StoryEvent) -> Result<(), SyncError>;

    fn mutate_batch_sync(&self, events: Vec<StoryEvent>) -> Result<(), SyncError>;
    fn issue_lock(&self, issue: u64) -> BoxedFuture<'_, Arc<tokio::sync::Mutex<()>>>;
    fn snapshot_status(&self) -> SyncStatus;
    fn acknowledge_dropped(&self) -> usize;
    fn drain_pending(&self) -> BoxedFuture<'_, Result<usize, SyncError>>;
    fn reconcile_dual_state_labels(&self) -> BoxedFuture<'_, Result<usize, SyncError>>;
    fn fetch_body(&self, id: u64) -> BoxedFuture<'_, Result<String, SyncError>>;
    fn fetch_body_conditional(
        &self,
        id: u64,
        _prior_etag: Option<String>,
    ) -> BoxedFuture<'_, Result<ConditionalBody, SyncError>> {
        Box::pin(async move {
            self.fetch_body(id)
                .await
                .map(|body| ConditionalBody::Modified { body, etag: None })
        })
    }
    fn fetch_free_comments(&self, id: u64) -> BoxedFuture<'_, Result<Vec<FreeComment>, SyncError>>;
    fn fetch_verdicts(&self, id: u64) -> BoxedFuture<'_, Result<Vec<CastVerdict>, SyncError>>;
    fn create_story(
        &self,
        title: String,
        body: String,
        initial_labels: Vec<String>,
    ) -> BoxedFuture<'_, Result<u64, SyncError>>;
    fn story(&self, id: u64) -> Option<Story>;
    fn story_fine_stage(&self, story: &Story) -> Option<Stage> {
        crate::board::fine_stage(story, &crate::board::BoardPlacements::default())
    }
    fn story_held_stage(&self, story: &Story) -> Option<workflow::StageId> {
        crate::board::held_stage(story, &crate::board::BoardPlacements::default())
    }
    fn all_stories(&self) -> Vec<Story>;
    fn evidence_for_story(&self, _id: u64) -> Vec<Evidence> {
        Vec::new()
    }
    fn outcomes(&self) -> Vec<Outcome> {
        Vec::new()
    }
    fn iterations(&self) -> Vec<Iteration> {
        Vec::new()
    }
    fn iteration(&self, iteration_id: IterationId) -> Option<Iteration> {
        self.iterations()
            .into_iter()
            .find(|iteration| iteration.id == iteration_id)
    }
    fn epics(&self) -> Vec<Epic> {
        Vec::new()
    }
    fn epic(&self, epic_id: EpicId) -> Option<Epic> {
        self.epics().into_iter().find(|epic| epic.id == epic_id)
    }
    fn backing_story(&self, id: u64) -> Option<Story> {
        self.story(id)
    }
    fn store_is_empty(&self) -> bool {
        self.all_stories().is_empty()
    }
    fn remove_story(&self, id: u64) -> Option<Story>;
    fn clear_stories(&self);
    fn update_body(&self, id: u64, body: String) -> BoxedFuture<'_, Result<(), SyncError>>;

    fn update_title(&self, id: u64, title: String) -> BoxedFuture<'_, Result<(), SyncError>>;

    fn ci_state(&self) -> Option<workflow::CiSnapshot> {
        None
    }

    fn test_race_window_rendezvous(&self) -> BoxedFuture<'_, ()> {
        Box::pin(async {})
    }

    fn test_race_window_complete(&self) {}
}
