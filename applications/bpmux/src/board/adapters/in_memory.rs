use std::collections::HashMap;
use std::sync::Mutex;

use crate::board::domain::{FreeComment, GateComment, Story};
use crate::board::events::BoardEvent;
use crate::board::sync::{SourceAdapter, SyncError};

#[derive(Default)]
pub struct InMemorySourceAdapter {
    state: Mutex<InnerState>,
}

#[derive(Default)]
struct InnerState {
    stories: HashMap<u64, Story>,
    bodies: HashMap<u64, String>,
    free_comments: HashMap<u64, Vec<FreeComment>>,
    gate_comments: HashMap<u64, Vec<GateComment>>,
    pushed_events: Vec<BoardEvent>,
    failures_on_push: bool,
    failures_on_ingest: bool,
}

impl InMemorySourceAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_stories(&self, stories: Vec<Story>) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        for story in stories {
            s.stories.insert(story.number, story);
        }
    }

    pub fn seed_body(&self, number: u64, body: String) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.bodies.insert(number, body);
    }

    pub fn seed_free_comments(&self, number: u64, comments: Vec<FreeComment>) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.free_comments.insert(number, comments);
    }

    pub fn seed_gate_comments(&self, number: u64, comments: Vec<GateComment>) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.gate_comments.insert(number, comments);
    }

    pub fn pushed_events(&self) -> Vec<BoardEvent> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pushed_events
            .clone()
    }

    pub fn set_push_failures(&self, fail: bool) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .failures_on_push = fail;
    }

    pub fn set_ingest_failures(&self, fail: bool) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .failures_on_ingest = fail;
    }
}

impl SourceAdapter for InMemorySourceAdapter {
    async fn ingest_metadata(&self) -> Result<Vec<Story>, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        Ok(s.stories.values().cloned().collect())
    }

    async fn fetch_story_metadata(&self, number: u64) -> Result<Option<Story>, SyncError> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_ingest {
            return Err(SyncError::SourceUnreachable(
                "in-memory test failure".into(),
            ));
        }
        Ok(s.stories.get(&number).cloned())
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

    async fn push_event(&self, event: &BoardEvent) -> Result<(), SyncError> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_push {
            return Err(SyncError::SourceUnreachable("push test failure".into()));
        }
        s.pushed_events.push(event.clone());
        Ok(())
    }

    async fn create_story(&self, title: &str, body: &str) -> Result<u64, SyncError> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_push {
            return Err(SyncError::SourceUnreachable("push test failure".into()));
        }
        let number = s.stories.keys().max().map(|n| n + 1).unwrap_or(1);
        let story = Story::new_minimal(
            number,
            title.to_string(),
            crate::board::domain::StoryState::Icebox,
            chrono::Utc::now(),
            "in-memory".to_string(),
        );
        s.stories.insert(number, story);
        if !body.is_empty() {
            s.bodies.insert(number, body.to_string());
        }
        Ok(number)
    }
}
