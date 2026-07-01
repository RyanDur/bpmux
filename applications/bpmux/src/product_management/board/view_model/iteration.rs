use serde::Serialize;

use crate::product_management::iteration::readiness::{IterationReadiness, iteration_readiness};
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::outcome::{Outcome, OutcomeId};
use crate::product_management::story::StoryId;
use crate::workflow::config::WorkflowConfig;

#[derive(Debug, Clone, Serialize)]
pub struct IterationView {
    pub id: u64,
    pub title: String,
    pub iteration_order: Option<u32>,
    pub is_current: bool,
    pub complete: bool,
    pub builds_outcome: Option<OutcomeId>,
    pub outcome_position: Option<u32>,
    pub stories: Vec<StoryId>,
    pub readiness: IterationReadiness,
}

impl IterationView {
    pub fn project(iteration: &Iteration, outcomes: &[Outcome], workflow: &WorkflowConfig) -> Self {
        let id = iteration.id().saved_id().map(|id| id.value()).unwrap_or(0);
        let outcome_position = iteration.builds_outcome().and_then(|outcome_id| {
            outcomes
                .iter()
                .find(|o| o.id == outcome_id)
                .and_then(|o| o.position_of(IterationId::new(id)))
                .map(|index| index + 1)
        });
        Self {
            id,
            title: iteration.content.title.clone(),
            iteration_order: iteration.iteration_order,
            is_current: iteration.is_current,
            complete: iteration.complete,
            builds_outcome: iteration.builds_outcome(),
            outcome_position,
            stories: iteration.stories.clone(),
            readiness: iteration_readiness(iteration, workflow),
        }
    }
}
