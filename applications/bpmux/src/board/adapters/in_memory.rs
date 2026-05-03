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
    labels: HashMap<u64, Vec<String>>,
    free_comments: HashMap<u64, Vec<FreeComment>>,
    gate_comments: HashMap<u64, Vec<GateComment>>,
    pushed_events: Vec<BoardEvent>,
    failures_on_push: bool,
    permanent_push_failures: bool,
    failures_on_ingest: bool,
}

impl InMemorySourceAdapter {
    pub fn new() -> Self {
        let env_fail = std::env::var("BPMUX_TEST_INGEST_FAIL")
            .map(|v| !v.is_empty() && v != "0")
            .unwrap_or(false);
        let mut adapter = Self::default();
        if env_fail {
            adapter
                .state
                .get_mut()
                .unwrap_or_else(|e| e.into_inner())
                .failures_on_ingest = true;
        }
        adapter
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

    pub fn seed_labels(&self, number: u64, labels: Vec<String>) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.labels.insert(number, labels);
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

    pub fn set_permanent_push_failures(&self, fail: bool) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .permanent_push_failures = fail;
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
        Ok(s.stories
            .values()
            .cloned()
            .map(|mut story| {
                if let Some(labels) = s.labels.get(&story.number) {
                    story.raw_labels = labels.clone();
                }
                story
            })
            .collect())
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
            story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
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
        match event {
            BoardEvent::FreeCommentAppended { number, body, meta } => {
                let entry = FreeComment {
                    body: body.clone(),
                    author: meta.actor.clone(),
                    ts: meta.ts,
                };
                s.free_comments.entry(*number).or_default().push(entry);
            }
            BoardEvent::GateCommentAppended {
                number,
                gate,
                payload,
                meta,
            } => {
                let entry = GateComment {
                    gate: *gate,
                    ts: meta.ts,
                    actor: meta.actor.clone(),
                    payload: payload.clone(),
                };
                s.gate_comments.entry(*number).or_default().push(entry);
            }
            BoardEvent::BodyFetched { number, body, .. } => {
                s.bodies.insert(*number, body.clone());
            }
            BoardEvent::TagAdded { number, label, .. } => {
                if label.eq_ignore_ascii_case("blocked")
                    && let Some(story) = s.stories.get_mut(number)
                    && !story.tags.contains(&crate::board::domain::Tag::Blocked)
                {
                    story.tags.push(crate::board::domain::Tag::Blocked);
                }
            }
            BoardEvent::TagRemoved { number, label, .. } => {
                if label.eq_ignore_ascii_case("blocked")
                    && let Some(story) = s.stories.get_mut(number)
                {
                    story
                        .tags
                        .retain(|t| *t != crate::board::domain::Tag::Blocked);
                }
            }
            BoardEvent::StoryClosed { number, .. } => {
                if let Some(story) = s.stories.get_mut(number) {
                    story.is_closed = true;
                }
            }
            BoardEvent::StoryReopened { number, .. } => {
                if let Some(story) = s.stories.get_mut(number) {
                    story.is_closed = false;
                }
            }
            _ => {}
        }
        // #1151: in-memory adapter doesn't have a GH-side response to
        // capture; the test substrate updates its own state via the
        // pushed_events log. Production github adapter will return
        // Some(Story) after the response-capture wiring lands.
        Ok(None)
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

    async fn update_body(&self, number: u64, body: &str) -> Result<Story, SyncError> {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.failures_on_push {
            return Err(SyncError::SourceUnreachable("push test failure".into()));
        }
        s.bodies.insert(number, body.to_string());
        // #1151: return the post-update Story (test substrate has
        // direct access to its own state; production github adapter
        // parses the GH-API response Issue).
        let mut story =
            s.stories.get(&number).cloned().ok_or_else(|| {
                SyncError::Other(format!("update_body: story #{number} not seeded"))
            })?;
        story.body = Some(body.to_string());
        story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
        Ok(story)
    }
}
