use std::collections::HashMap;
use std::sync::RwLock;

use super::domain::{FreeComment, GateComment, Story, StoryState};
use super::events::BoardEvent;

pub struct BoardStore {
    stories: RwLock<HashMap<u64, Story>>,
}

impl BoardStore {
    pub fn new() -> Self {
        Self {
            stories: RwLock::new(HashMap::new()),
        }
    }

    pub fn seed_all(&self, stories: Vec<Story>) {
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        for story in stories {
            w.insert(story.number, story);
        }
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
            } => {
                let story = Story {
                    number: *n,
                    title: title.clone(),
                    state: *initial_state,
                    filed_at: meta.ts,
                    filed_by: meta.actor.clone(),
                    body: Some(body.clone()),
                    free_comments: Some(Vec::new()),
                    gate_comments: Vec::new(),
                };
                w.insert(*n, story);
            }
            BoardEvent::StateTransitioned { new_state, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.state = *new_state;
                }
            }
            BoardEvent::GateCommentAppended {
                meta,
                gate,
                payload,
                ..
            } => {
                if let Some(story) = w.get_mut(&number) {
                    story
                        .gate_comments
                        .push(GateComment::from_event(*gate, meta, payload.clone()));
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
                }
            }
            BoardEvent::FreeCommentsFetched { comments, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.free_comments = Some(comments.clone());
                }
            }
        }
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
