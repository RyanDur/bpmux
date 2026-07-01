use super::SyncOrchestrator;
use super::status::{record_error, record_ingest_success};
use crate::product_management::board::events::BoardEvent;
use crate::product_management::board::sync::{SourceAdapter, SyncError};
use crate::product_management::story::{Story, StoryId};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn ingest_once(&self) -> Result<(), SyncError> {
        self.ingest_once_since(None).await
    }

    pub async fn ingest_once_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), SyncError> {
        match self.adapter.ingest_metadata_since(since).await {
            Ok(ingested) => {
                let crate::product_management::board::sync::Ingested { items, outcomes } = ingested;
                let in_flight_label_stories = self.with_pending(|p| {
                    p.iter()
                        .filter(|q| {
                            matches!(
                                q.event,
                                BoardEvent::TagAdded { .. } | BoardEvent::TagRemoved { .. }
                            )
                        })
                        .map(|q| q.event.story_id())
                        .collect::<Vec<StoryId>>()
                });
                let items = items
                    .into_iter()
                    .filter(|item| {
                        item.id()
                            .saved_id()
                            .map(|id| !in_flight_label_stories.contains(&id))
                            .unwrap_or(true)
                    })
                    .collect::<Vec<_>>();
                let fresh_outcomes = outcomes
                    .into_iter()
                    .filter(|outcome| {
                        !in_flight_label_stories
                            .iter()
                            .any(|id| id.value() == outcome.id.value())
                    })
                    .collect::<Vec<_>>();
                let backing: Vec<Story> = items
                    .iter()
                    .filter_map(|i| i.backing_story().cloned())
                    .collect();
                let purge =
                    crate::product_management::board::adapters::github::iterations_to_purge_as_retired_empty(
                        &backing,
                    );
                if let Some(tx) = &self.action_tx
                    && !fresh_outcomes.is_empty()
                {
                    let _ = tx.send(crate::store::AppAction::Outcome(
                        crate::product_management::outcome::OutcomeAction::Upserted(fresh_outcomes),
                    ));
                }
                self.store.seed_items(items);
                for iteration_id in purge {
                    self.store.remove(StoryId::new(iteration_id.value()));
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
    ) -> Result<Option<crate::product_management::story::Story>, SyncError> {
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
}
