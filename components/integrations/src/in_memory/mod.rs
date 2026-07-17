use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

use product_management::story::events::StoryEvent;
use product_management::story::{CastVerdict, FreeComment, Story};

mod ingest;
mod mutation;
mod seed;
mod source_adapter;
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
    push_follow_ups: VecDeque<StoryEvent>,
    failures_on_push: bool,
    permanent_push_failures: bool,
    failures_on_ingest: bool,
}
