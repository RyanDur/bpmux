pub mod label;
pub mod readiness;
pub mod section;

use crate::product_management::board::item::Content;
use crate::product_management::outcome::OutcomeId;
use crate::product_management::story::{StoryId, StoryKey};

entity_id!(IterationId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iteration {
    pub content: Content,
    pub outcome: Option<OutcomeId>,
    pub iteration_order: Option<u32>,
    pub stories: Vec<StoryId>,
    pub review_closers: Vec<String>,
    pub dev_ok_by: Vec<String>,
    pub is_current: bool,
    pub complete: bool,
}

impl Iteration {
    pub fn id(&self) -> StoryKey {
        self.content.id
    }

    pub fn builds_outcome(&self) -> Option<OutcomeId> {
        self.outcome
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
