use super::SyncOrchestrator;
use super::status::{record_error, record_ingest_success};
use crate::product_management::board::events::BoardEvent;
use crate::product_management::board::sync::{SourceAdapter, SyncError};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn ingest_once(&self) -> Result<(), SyncError> {
        self.ingest_once_since(None).await
    }

    pub async fn ingest_once_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), SyncError> {
        match self.adapter.ingest_metadata_since(since).await {
            Ok(stories) => {
                let in_flight_label_stories = self.with_pending(|p| {
                    p.iter()
                        .filter(|q| {
                            matches!(
                                q.event,
                                BoardEvent::TagAdded { .. } | BoardEvent::TagRemoved { .. }
                            )
                        })
                        .map(|q| q.event.story_number())
                        .collect::<Vec<u64>>()
                });
                let stories = stories
                    .into_iter()
                    .filter(|s| !in_flight_label_stories.contains(&s.number))
                    .collect::<Vec<_>>();
                let purge =
                    crate::product_management::board::adapters::github::iteration_numbers_to_purge_as_retired_empty(
                        &stories,
                    );
                self.store.seed_all(stories);
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
