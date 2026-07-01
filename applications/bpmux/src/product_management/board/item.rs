use chrono::{DateTime, Utc};

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
pub enum ProductItem {
    Story(Story),
}

impl ProductItem {
    pub fn from_story(story: &Story) -> ProductItem {
        ProductItem::Story(story.clone())
    }

    pub fn id(&self) -> StoryKey {
        match self {
            ProductItem::Story(story) => story.key(),
        }
    }

    pub fn as_story(&self) -> Option<&Story> {
        match self {
            ProductItem::Story(story) => Some(story),
        }
    }

    pub fn as_story_mut(&mut self) -> Option<&mut Story> {
        match self {
            ProductItem::Story(story) => Some(story),
        }
    }

    pub fn backing_story(&self) -> Option<&Story> {
        match self {
            ProductItem::Story(story) => Some(story),
        }
    }

    pub fn backing_story_mut(&mut self) -> Option<&mut Story> {
        match self {
            ProductItem::Story(story) => Some(story),
        }
    }

    pub fn current_labels(&self) -> Vec<String> {
        match self {
            ProductItem::Story(story) => story.raw_labels().to_vec(),
        }
    }
}
