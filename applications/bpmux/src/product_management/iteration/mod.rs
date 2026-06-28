pub mod label;
pub mod readiness;
pub mod section;

use crate::product_management::board::adapters::github::{
    is_current_from_labels, iteration_dev_ok_by_from_labels, iteration_order_from_labels,
    iteration_review_closers_from_labels, outcome_membership_from_labels,
};
use crate::product_management::board::item::Content;
use crate::product_management::story::{Story, StoryKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iteration {
    pub content: Content,
    pub outcome_membership: Option<(u64, u32)>,
    pub iteration_order: Option<u32>,
    pub review_closers: Vec<String>,
    pub dev_ok_by: Vec<String>,
    pub is_current: bool,
    pub complete: bool,
}

impl Iteration {
    pub fn from_story(story: &Story) -> Self {
        let labels = story.raw_labels();
        Iteration {
            content: Content {
                id: story.id(),
                title: story.title().to_string(),
                body: story.body().map(str::to_string),
                filed_at: story.filed_at(),
                filed_by: story.filed_by().to_string(),
            },
            outcome_membership: outcome_membership_from_labels(labels),
            iteration_order: iteration_order_from_labels(labels),
            review_closers: iteration_review_closers_from_labels(labels),
            dev_ok_by: iteration_dev_ok_by_from_labels(labels),
            is_current: is_current_from_labels(labels),
            complete: story.is_closed,
        }
    }

    pub fn id(&self) -> StoryKey {
        self.content.id
    }

    pub fn builds_outcome(&self) -> Option<u64> {
        self.outcome_membership.map(|(outcome, _)| outcome)
    }
}

#[cfg(test)]
mod iteration_tests;
#[cfg(test)]
mod label_tests;
#[cfg(test)]
mod readiness_tests;
#[cfg(test)]
mod section_tests;
