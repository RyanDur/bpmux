use std::collections::HashMap;
use std::sync::Mutex;

use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{CastVerdict, FreeComment, Story, StoryId};
use crate::sync::{Ingested, SourceAdapter, SyncError};

mod ingest;
mod mutation;
mod seed;
mod typed_label_mirror;

#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct InMemorySourceAdapter {
    state: Mutex<InnerState>,
}

#[derive(Default)]
struct InnerState {
    stories: HashMap<u64, Story>,
    bodies: HashMap<u64, String>,
    labels: HashMap<u64, Vec<String>>,
    free_comments: HashMap<u64, Vec<FreeComment>>,
    verdicts: HashMap<u64, Vec<CastVerdict>>,
    closed: std::collections::HashSet<u64>,
    pushed_events: Vec<StoryEvent>,
    failures_on_push: bool,
    permanent_push_failures: bool,
    failures_on_ingest: bool,
}

impl SourceAdapter for InMemorySourceAdapter {
    async fn ingest_metadata(&self) -> Result<Ingested, SyncError> {
        ingest::ingest_metadata(&self.state)
    }

    async fn fetch_story_metadata(
        &self,
        number: u64,
    ) -> Result<Option<(Story, Option<crate::workflow::StageId>)>, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        let Some(mut story) = s.stories.get(&number).cloned() else {
            return Ok(None);
        };
        let labels = s.labels.get(&number).cloned().unwrap_or_default();
        ingest::derive_typed_fields_from_labels(&mut story, &labels);
        if let Some(comments) = s.free_comments.get(&number) {
            story.free_comments = Some(comments.clone());
        }
        if let Some(comments) = s.verdicts.get(&number) {
            story.verdicts = Some(comments.clone());
        }
        if let Some(body) = s.bodies.get(&number) {
            story.body = Some(body.clone());
        }
        let placement = crate::integrations::github::mapper::placement_from_labels(
            &labels,
            s.closed.contains(&number),
        );
        Ok(Some((story, placement)))
    }

    async fn fetch_story_body(&self, number: u64) -> Result<String, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        s.bodies
            .get(&number)
            .cloned()
            .ok_or_else(|| SyncError::Other(format!("no body for #{number}")))
    }

    async fn fetch_story_labels(&self, number: u64) -> Result<Vec<String>, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        Ok(s.labels.get(&number).cloned().unwrap_or_default())
    }

    async fn fetch_free_comments(&self, number: u64) -> Result<Vec<FreeComment>, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        Ok(s.free_comments.get(&number).cloned().unwrap_or_default())
    }

    async fn fetch_verdicts(&self, number: u64) -> Result<Vec<CastVerdict>, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        Ok(s.verdicts.get(&number).cloned().unwrap_or_default())
    }

    async fn push_event(
        &self,
        event: &StoryEvent,
        context: &crate::sync::PushContext,
    ) -> Result<Option<StoryEvent>, SyncError> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.permanent_push_failures {
            return Err(SyncError::ValidationRejected(
                "permanent test failure".into(),
            ));
        }
        if s.failures_on_push {
            return Err(SyncError::SourceUnreachable("push test failure".into()));
        }
        s.pushed_events.push(event.clone());
        mutation::apply_push_event(&mut s, event, context);
        Ok(None)
    }

    async fn create_story(
        &self,
        title: &str,
        body: &str,
        initial_labels: &[&str],
    ) -> Result<u64, SyncError> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_push {
            return Err(SyncError::SourceUnreachable("push test failure".into()));
        }
        let number = s.stories.keys().max().map(|n| n + 1).unwrap_or(1);
        let story = Story::new_minimal(
            StoryId::new(number),
            title.to_string(),
            crate::product_management::story::Work::NotStarted,
            chrono::Utc::now(),
            "in-memory".to_string(),
        );
        s.stories.insert(number, story);
        if !body.is_empty() {
            s.bodies.insert(number, body.to_string());
        }
        if !initial_labels.is_empty() {
            s.labels.insert(
                number,
                initial_labels.iter().map(|l| l.to_string()).collect(),
            );
        }
        Ok(number)
    }
}
