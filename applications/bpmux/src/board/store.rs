use std::collections::HashMap;
use std::sync::RwLock;

use tokio::sync::broadcast;

use super::domain::{FreeComment, GateComment, Story, StoryState, Tag};
use super::events::BoardEvent;

fn tag_from_label_string(s: &str) -> Option<Tag> {
    if s.eq_ignore_ascii_case("blocked") {
        Some(Tag::Blocked)
    } else {
        None
    }
}

const SUBSCRIBE_BUFFER: usize = 1_000;

pub struct BoardStore {
    stories: RwLock<HashMap<u64, Story>>,
    event_tx: broadcast::Sender<BoardEvent>,
}

impl BoardStore {
    pub fn new() -> Self {
        let (event_tx, _rx) = broadcast::channel(SUBSCRIBE_BUFFER);
        Self {
            stories: RwLock::new(HashMap::new()),
            event_tx,
        }
    }

    pub fn seed_all(&self, stories: Vec<Story>) {
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        for story in stories {
            w.insert(story.number, story);
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BoardEvent> {
        self.event_tx.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.event_tx.receiver_count()
    }

    pub fn apply(&self, event: &BoardEvent) {
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        let number = event.story_number();
        match event {
            BoardEvent::StoryCreated {
                meta,
                number: n,
                title,
                body,
                initial_state,
                story_type,
            } => {
                let story = Story {
                    number: *n,
                    title: title.clone(),
                    state: *initial_state,
                    story_type: *story_type,
                    tags: Vec::new(),
                    filed_at: meta.ts,
                    filed_by: meta.actor.clone(),
                    body: Some(body.clone()),
                    free_comments: Some(Vec::new()),
                    gate_comments: Some(Vec::new()),
                    is_closed: false,
                    parent_epic: None,
                    acceptance_test_path: crate::bridge::parse_acceptance_test_path(body),
                };
                w.insert(*n, story);
            }
            BoardEvent::StateTransitioned { new_state, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.state = *new_state;
                }
            }
            BoardEvent::TagAdded { label, .. } => {
                if let Some(tag) = tag_from_label_string(label)
                    && let Some(story) = w.get_mut(&number)
                    && !story.tags.contains(&tag)
                {
                    story.tags.push(tag);
                }
            }
            BoardEvent::TagRemoved { label, .. } => {
                if let Some(tag) = tag_from_label_string(label)
                    && let Some(story) = w.get_mut(&number)
                {
                    story.tags.retain(|t| *t != tag);
                }
            }
            BoardEvent::GateCommentAppended {
                meta,
                gate,
                payload,
                ..
            } => {
                if let Some(story) = w.get_mut(&number) {
                    let entry = GateComment::from_event(*gate, meta, payload.clone());
                    match &mut story.gate_comments {
                        Some(list) => list.push(entry),
                        None => story.gate_comments = Some(vec![entry]),
                    }
                }
            }
            BoardEvent::FreeCommentAppended { meta, body, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    let entry = FreeComment::from_event(body.clone(), meta);
                    match &mut story.free_comments {
                        Some(list) => list.push(entry),
                        None => story.free_comments = Some(vec![entry]),
                    }
                }
            }
            BoardEvent::BodyFetched { body, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.body = Some(body.clone());
                    story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
                }
            }
            BoardEvent::FreeCommentsFetched { comments, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.free_comments = Some(comments.clone());
                }
            }
            BoardEvent::GateCommentsFetched { comments, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.gate_comments = Some(comments.clone());
                }
            }
            BoardEvent::StoryClosed { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.is_closed = true;
                }
            }
            BoardEvent::ParentEpicSet { epic_number, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.parent_epic = Some(*epic_number);
                }
            }
            BoardEvent::ParentEpicUnset { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.parent_epic = None;
                }
            }
        }
        drop(w);
        let _ = self.event_tx.send(event.clone());
    }

    pub fn story(&self, number: u64) -> Option<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&number)
            .cloned()
    }

    pub fn all_stories(&self) -> Vec<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    pub fn stories_in_state(&self, state: StoryState) -> Vec<Story> {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|s| s.state == state)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.stories.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.stories
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
    }
}

impl Default for BoardStore {
    fn default() -> Self {
        Self::new()
    }
}
