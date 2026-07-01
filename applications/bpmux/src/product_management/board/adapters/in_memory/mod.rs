use std::collections::HashMap;
use std::sync::Mutex;

use crate::product_management::board::events::BoardEvent;
use crate::product_management::board::sync::{Ingested, SourceAdapter, SyncError};
use crate::product_management::story::{FreeComment, GateComment, Story, StoryId};

mod ingest;
mod mutation;
mod seed;

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
    gate_comments: HashMap<u64, Vec<GateComment>>,
    pushed_events: Vec<BoardEvent>,
    failures_on_push: bool,
    permanent_push_failures: bool,
    failures_on_ingest: bool,
}

impl SourceAdapter for InMemorySourceAdapter {
    async fn ingest_metadata(&self) -> Result<Ingested, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        let ingest::Classified {
            items,
            outcomes,
            mut iterations,
            epics,
        } = ingest::classify_ingested(s.stories.values().cloned(), &s.labels);
        iterations.retain(|iteration| !(iteration.complete && iteration.stories.is_empty()));
        Ok(Ingested {
            items,
            outcomes,
            iterations,
            epics,
        })
    }

    async fn fetch_story_metadata(&self, number: u64) -> Result<Option<Story>, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        let Some(mut story) = s.stories.get(&number).cloned() else {
            return Ok(None);
        };
        if let Some(labels) = s.labels.get(&number) {
            story.raw_labels = labels.clone();
        }
        if let Some(comments) = s.free_comments.get(&number) {
            story.free_comments = Some(comments.clone());
        }
        if let Some(comments) = s.gate_comments.get(&number) {
            story.gate_comments = Some(comments.clone());
        }
        if let Some(body) = s.bodies.get(&number) {
            story.body = Some(body.clone());
        }
        Ok(Some(story))
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

    async fn fetch_gate_comments(&self, number: u64) -> Result<Vec<GateComment>, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        Ok(s.gate_comments.get(&number).cloned().unwrap_or_default())
    }

    async fn push_event(&self, event: &BoardEvent) -> Result<Option<BoardEvent>, SyncError> {
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
        mutation::apply_push_event(&mut s, event);
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
            crate::product_management::story::BoardState::Icebox,
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

    async fn update_body(&self, number: u64, body: &str) -> Result<Story, SyncError> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_push {
            return Err(SyncError::SourceUnreachable("push test failure".into()));
        }
        s.bodies.insert(number, body.to_string());
        let mut story =
            s.stories.get(&number).cloned().ok_or_else(|| {
                SyncError::Other(format!("update_body: story #{number} not seeded"))
            })?;
        story.body = Some(body.to_string());
        s.stories.insert(number, story.clone());
        Ok(story)
    }

    async fn update_title(&self, number: u64, title: &str) -> Result<Story, SyncError> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_push {
            return Err(SyncError::SourceUnreachable("push test failure".into()));
        }
        let mut story =
            s.stories.get(&number).cloned().ok_or_else(|| {
                SyncError::Other(format!("update_title: story #{number} not seeded"))
            })?;
        story.title = title.to_string();
        s.stories.insert(number, story.clone());
        Ok(story)
    }
}
