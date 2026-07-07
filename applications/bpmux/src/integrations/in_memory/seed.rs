use super::InMemorySourceAdapter;
use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{CastVerdict, FreeComment, Story};

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
            s.stories.insert(story.id.value(), story);
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

    pub fn seed_closed(&self, number: u64) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.closed.insert(number);
    }

    pub fn seed_free_comments(&self, number: u64, comments: Vec<FreeComment>) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.free_comments.insert(number, comments);
    }

    pub fn seed_verdicts(&self, number: u64, verdicts: Vec<CastVerdict>) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.verdicts.insert(number, verdicts);
    }

    pub fn pushed_events(&self) -> Vec<StoryEvent> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pushed_events
            .clone()
    }

    pub fn labels_for(&self, number: u64) -> Vec<String> {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.labels.get(&number).cloned().unwrap_or_default()
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
