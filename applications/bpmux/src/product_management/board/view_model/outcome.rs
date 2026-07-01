use serde::Serialize;

use crate::product_management::iteration::Iteration;
use crate::product_management::iteration::readiness::{
    IterationReadiness, OutcomeReadiness, building_iterations, iteration_readiness,
    outcome_readiness,
};
use crate::product_management::outcome::Outcome;
use crate::workflow::config::WorkflowConfig;

#[derive(Debug, Clone, Serialize)]
pub struct BuildingIterationView {
    pub id: u64,
    pub name: String,
    pub position: u32,
    pub readiness: IterationReadiness,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutcomeView {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub horizon: Option<&'static str>,
    pub horizon_order: Option<u32>,
    pub outcome_readiness: OutcomeReadiness,
    pub iterations: Vec<BuildingIterationView>,
}

impl OutcomeView {
    pub fn project(
        outcome: &Outcome,
        all_iterations: &[Iteration],
        workflow: &WorkflowConfig,
    ) -> Self {
        let iterations = building_iterations(outcome, all_iterations)
            .iter()
            .enumerate()
            .map(|(index, iteration)| BuildingIterationView {
                id: iteration.id().saved_id().map(|id| id.value()).unwrap_or(0),
                name: iteration.content.title.clone(),
                position: index as u32 + 1,
                readiness: iteration_readiness(iteration, workflow),
            })
            .collect();
        Self {
            id: outcome.id.value(),
            title: outcome.title.clone(),
            body: outcome.body.clone().unwrap_or_default(),
            horizon: outcome.horizon().map(|horizon| horizon.as_str()),
            horizon_order: outcome.horizon_order(),
            outcome_readiness: outcome_readiness(outcome, all_iterations, workflow),
            iterations,
        }
    }
}
