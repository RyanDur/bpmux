use chrono::{DateTime, Utc};

use crate::product_management::story::{Story, StoryKey, StoryType};

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
    Iteration(Story),
    Epic(Story),
}

impl ProductItem {
    pub fn from_story(story: &Story) -> ProductItem {
        match story.story_type() {
            StoryType::Iteration => ProductItem::Iteration(story.clone()),
            StoryType::Epic => ProductItem::Epic(story.clone()),
            StoryType::Story(_) => ProductItem::Story(story.clone()),
        }
    }

    pub fn id(&self) -> StoryKey {
        match self {
            ProductItem::Story(story)
            | ProductItem::Iteration(story)
            | ProductItem::Epic(story) => story.key(),
        }
    }

    pub fn as_story(&self) -> Option<&Story> {
        match self {
            ProductItem::Story(story) => Some(story),
            ProductItem::Iteration(_) | ProductItem::Epic(_) => None,
        }
    }

    pub fn as_story_mut(&mut self) -> Option<&mut Story> {
        match self {
            ProductItem::Story(story) => Some(story),
            ProductItem::Iteration(_) | ProductItem::Epic(_) => None,
        }
    }

    pub fn iteration_story(&self) -> Option<&Story> {
        match self {
            ProductItem::Iteration(story) => Some(story),
            ProductItem::Story(_) | ProductItem::Epic(_) => None,
        }
    }

    pub fn epic_story(&self) -> Option<&Story> {
        match self {
            ProductItem::Epic(story) => Some(story),
            ProductItem::Story(_) | ProductItem::Iteration(_) => None,
        }
    }

    pub fn backing_story(&self) -> Option<&Story> {
        match self {
            ProductItem::Story(story)
            | ProductItem::Iteration(story)
            | ProductItem::Epic(story) => Some(story),
        }
    }

    pub fn backing_story_mut(&mut self) -> Option<&mut Story> {
        match self {
            ProductItem::Story(story)
            | ProductItem::Iteration(story)
            | ProductItem::Epic(story) => Some(story),
        }
    }

    pub fn current_labels(&self) -> Vec<String> {
        match self {
            ProductItem::Story(story)
            | ProductItem::Iteration(story)
            | ProductItem::Epic(story) => story.raw_labels().to_vec(),
        }
    }
}
