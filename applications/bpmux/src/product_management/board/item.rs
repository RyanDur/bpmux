use chrono::{DateTime, Utc};

use crate::product_management::outcome::Outcome;
use crate::product_management::story::{Story, StoryKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub id: StoryKey,
    pub title: String,
    pub body: Option<String>,
    pub filed_at: DateTime<Utc>,
    pub filed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardItem {
    Story(Story),
    Outcome(Outcome),
}

impl BoardItem {
    pub fn id(&self) -> StoryKey {
        match self {
            BoardItem::Story(story) => story.id(),
            BoardItem::Outcome(outcome) => outcome.id(),
        }
    }

    pub fn as_story(&self) -> Option<&Story> {
        match self {
            BoardItem::Story(story) => Some(story),
            BoardItem::Outcome(_) => None,
        }
    }

    pub fn as_story_mut(&mut self) -> Option<&mut Story> {
        match self {
            BoardItem::Story(story) => Some(story),
            BoardItem::Outcome(_) => None,
        }
    }

    pub fn as_outcome(&self) -> Option<&Outcome> {
        match self {
            BoardItem::Outcome(outcome) => Some(outcome),
            BoardItem::Story(_) => None,
        }
    }

    pub fn as_outcome_mut(&mut self) -> Option<&mut Outcome> {
        match self {
            BoardItem::Outcome(outcome) => Some(outcome),
            BoardItem::Story(_) => None,
        }
    }

    pub fn current_labels(&self) -> Vec<String> {
        match self {
            BoardItem::Story(story) => story.raw_labels().to_vec(),
            BoardItem::Outcome(outcome) => outcome.encoded_labels(),
        }
    }
}
