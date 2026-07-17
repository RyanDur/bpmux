use product_management::story::events::StoryEvent;
use product_management::story::{CastVerdict, FreeComment, Story, StoryId};
use source_sync::{BoxedFuture, Ingested, SourceAdapter, StoryWithPlacement, SyncError};

use super::{InMemorySourceAdapter, ingest, mutation};

impl SourceAdapter for InMemorySourceAdapter {
    fn ingest_metadata(&self) -> BoxedFuture<'_, Result<Ingested, SyncError>> {
        Box::pin(async move { ingest::ingest_metadata(&self.state) })
    }

    fn fetch_story_metadata(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Option<StoryWithPlacement>, SyncError>> {
        Box::pin(async move {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.failures_on_ingest {
                return Err(SyncError::SourceUnreachable(
                    "in-memory test failure".into(),
                ));
            }
            let Some(mut story) = state.stories.get(&number).cloned() else {
                return Ok(None);
            };
            let labels = state.labels.get(&number).cloned().unwrap_or_default();
            ingest::derive_typed_fields_from_labels(&mut story, &labels);
            if let Some(comments) = state.free_comments.get(&number) {
                story.free_comments = Some(comments.clone());
            }
            if let Some(comments) = state.verdicts.get(&number) {
                story.verdicts = Some(comments.clone());
            }
            if let Some(body) = state.bodies.get(&number) {
                story.body = Some(body.clone());
            }
            let placement = source_codec::mapper::placement_from_labels(
                &labels,
                state.closed.contains(&number),
            );
            Ok(Some((story, placement)))
        })
    }

    fn fetch_story_body(&self, number: u64) -> BoxedFuture<'_, Result<String, SyncError>> {
        Box::pin(async move {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.failures_on_ingest {
                return Err(SyncError::SourceUnreachable(
                    "in-memory test failure".into(),
                ));
            }
            state
                .bodies
                .get(&number)
                .cloned()
                .ok_or_else(|| SyncError::Other(format!("no body for #{number}")))
        })
    }

    fn fetch_story_labels(&self, number: u64) -> BoxedFuture<'_, Result<Vec<String>, SyncError>> {
        Box::pin(async move {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.failures_on_ingest {
                return Err(SyncError::SourceUnreachable(
                    "in-memory test failure".into(),
                ));
            }
            Ok(state.labels.get(&number).cloned().unwrap_or_default())
        })
    }

    fn fetch_free_comments(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<FreeComment>, SyncError>> {
        Box::pin(async move {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.failures_on_ingest {
                return Err(SyncError::SourceUnreachable(
                    "in-memory test failure".into(),
                ));
            }
            Ok(state
                .free_comments
                .get(&number)
                .cloned()
                .unwrap_or_default())
        })
    }

    fn fetch_verdicts(&self, number: u64) -> BoxedFuture<'_, Result<Vec<CastVerdict>, SyncError>> {
        Box::pin(async move {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.failures_on_ingest {
                return Err(SyncError::SourceUnreachable(
                    "in-memory test failure".into(),
                ));
            }
            Ok(state.verdicts.get(&number).cloned().unwrap_or_default())
        })
    }

    fn push_event<'a>(
        &'a self,
        event: &'a StoryEvent,
        context: &'a source_sync::PushContext,
    ) -> BoxedFuture<'a, Result<Option<StoryEvent>, SyncError>> {
        Box::pin(async move {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.permanent_push_failures {
                return Err(SyncError::ValidationRejected(
                    "permanent test failure".into(),
                ));
            }
            if state.failures_on_push {
                return Err(SyncError::SourceUnreachable("push test failure".into()));
            }
            state.pushed_events.push(event.clone());
            mutation::apply_push_event(&mut state, event, context);
            Ok(state.push_follow_ups.pop_front())
        })
    }

    fn create_story<'a>(
        &'a self,
        title: &'a str,
        body: &'a str,
        initial_labels: &'a [&'a str],
    ) -> BoxedFuture<'a, Result<u64, SyncError>> {
        Box::pin(async move {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.failures_on_push {
                return Err(SyncError::SourceUnreachable("push test failure".into()));
            }
            let number = state
                .stories
                .keys()
                .max()
                .map(|highest| highest + 1)
                .unwrap_or(1);
            let story = Story::new_minimal(
                StoryId::new(number),
                title.to_string(),
                product_management::story::Work::NotStarted,
                chrono::Utc::now(),
                "in-memory".to_string(),
            );
            state.stories.insert(number, story);
            if !body.is_empty() {
                state.bodies.insert(number, body.to_string());
            }
            if !initial_labels.is_empty() {
                state.labels.insert(
                    number,
                    initial_labels
                        .iter()
                        .map(|label| label.to_string())
                        .collect(),
                );
            }
            Ok(number)
        })
    }
}
