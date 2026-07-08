use super::SyncOrchestrator;
use super::status::{record_error, record_ingest_success};
use crate::sync::{SourceAdapter, SyncError};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn ingest_once(&self) -> Result<(), SyncError> {
        self.ingest_once_since(None).await
    }

    fn seed_and_place(
        &self,
        items: Vec<crate::product_management::story::Story>,
        placements: Vec<(
            crate::product_management::story::StoryId,
            crate::workflow::StageId,
        )>,
    ) {
        let board_actions = crate::board::placements_for_ingest(&items, &placements);
        self.dispatch_story(crate::product_management::story::slice::StoryAction::Seeded(items));
        for action in board_actions {
            self.dispatch_board(crate::store::AppAction::Board(action));
        }
    }

    pub async fn ingest_once_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), SyncError> {
        match self.adapter.ingest_metadata_since(since).await {
            Ok(ingested) => {
                let crate::sync::Ingested {
                    items,
                    outcomes,
                    iterations,
                    epics,
                    placements,
                } = ingested;
                let stories_with_queued_local_writes = self.read_queued_story_ids();
                let not_in_flight = |value: u64| {
                    !stories_with_queued_local_writes
                        .iter()
                        .any(|id| id.value() == value)
                };
                let items = items
                    .into_iter()
                    .filter(|item| {
                        item.key()
                            .saved_id()
                            .map(|id| not_in_flight(id.value()))
                            .unwrap_or(true)
                    })
                    .collect::<Vec<_>>();
                let fresh_outcomes = outcomes
                    .into_iter()
                    .filter(|outcome| not_in_flight(outcome.id.value()))
                    .collect::<Vec<_>>();
                let fresh_iterations = iterations
                    .into_iter()
                    .filter(|iteration| not_in_flight(iteration.id.value()))
                    .collect::<Vec<_>>();
                let fresh_epics = epics
                    .into_iter()
                    .filter(|epic| not_in_flight(epic.id.value()))
                    .collect::<Vec<_>>();
                if !fresh_outcomes.is_empty() {
                    self.dispatch_board(crate::store::AppAction::Outcome(
                        crate::product_management::outcome::OutcomeAction::Upserted(fresh_outcomes),
                    ));
                }
                if !fresh_iterations.is_empty() {
                    self.dispatch_board(crate::store::AppAction::Iteration(
                        crate::product_management::iteration::IterationAction::Upserted(
                            fresh_iterations,
                        ),
                    ));
                }
                if !fresh_epics.is_empty() {
                    self.dispatch_board(crate::store::AppAction::Epic(
                        crate::product_management::epic::EpicAction::Upserted(fresh_epics),
                    ));
                }
                self.seed_and_place(items, placements);
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
    ) -> Result<Option<crate::product_management::story::Story>, SyncError> {
        match self.adapter.fetch_story_metadata(number).await {
            Ok(Some((story, placement))) => {
                let present_with_queued_local_write =
                    self.read_story(story.id).is_some() && self.read_queued_for_story(story.id) > 0;
                if present_with_queued_local_write {
                    crate::diag::log_diag(&format!(
                        "orchestrator: refresh seed for #{} deferred to a queued local write on the present story",
                        story.id.value()
                    ));
                } else {
                    let placements = placement
                        .map(|stage| (story.id, stage))
                        .into_iter()
                        .collect::<Vec<_>>();
                    self.seed_and_place(vec![story.clone()], placements);
                }
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
}
