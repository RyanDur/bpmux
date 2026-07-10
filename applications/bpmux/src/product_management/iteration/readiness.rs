use serde::Serialize;

use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;
use crate::workflow::config::WorkflowConfig;

pub const ITERATION_REVIEWER_PURPOSE: &str = "iteration-reviewer";
pub const ITERATION_BUILDER_PURPOSE: &str = "iteration-builder";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IterationReadiness {
    pub drafted: bool,
    pub stories_ordered: bool,
    pub review_closed: bool,
    pub dev_ok: bool,
}

impl IterationReadiness {
    pub fn all_met(&self) -> bool {
        self.drafted && self.stories_ordered && self.review_closed && self.dev_ok
    }

    pub fn unmet_count(&self) -> u32 {
        u32::from(!self.drafted)
            + u32::from(!self.stories_ordered)
            + u32::from(!self.review_closed)
            + u32::from(!self.dev_ok)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OutcomeReadiness {
    pub iteration_count: u32,
    pub gap_count: u32,
    pub all_ready: bool,
}

pub fn building_iterations(outcome: &Outcome, iterations: &[Iteration]) -> Vec<Iteration> {
    outcome
        .iterations
        .iter()
        .filter_map(|iteration_id| {
            iterations
                .iter()
                .find(|iteration| iteration.id == *iteration_id)
                .cloned()
        })
        .collect()
}

pub fn outcome_readiness(
    outcome: &Outcome,
    iterations: &[Iteration],
    workflow: &WorkflowConfig,
) -> OutcomeReadiness {
    let building = building_iterations(outcome, iterations);
    let gap_count = building
        .iter()
        .map(|iteration| iteration_readiness(iteration, workflow).unmet_count())
        .sum();
    let iteration_count = building.len() as u32;
    OutcomeReadiness {
        iteration_count,
        gap_count,
        all_ready: iteration_count > 0 && gap_count == 0,
    }
}

pub fn iteration_readiness(iteration: &Iteration, workflow: &WorkflowConfig) -> IterationReadiness {
    IterationReadiness {
        drafted: drafted(iteration),
        stories_ordered: stories_ordered(iteration),
        review_closed: review_closed(iteration, workflow),
        dev_ok: dev_ok(iteration, workflow),
    }
}

fn drafted(iteration: &Iteration) -> bool {
    iteration
        .body
        .as_deref()
        .is_some_and(|b| !b.trim().is_empty())
}

fn stories_ordered(iteration: &Iteration) -> bool {
    !iteration.stories.is_empty()
}

fn review_closed(iteration: &Iteration, workflow: &WorkflowConfig) -> bool {
    let reviewers = workflow.role_names_for_purpose(ITERATION_REVIEWER_PURPOSE);
    iteration
        .review_closers
        .iter()
        .any(|closer| reviewers.iter().any(|r| r == closer))
}

fn dev_ok(iteration: &Iteration, workflow: &WorkflowConfig) -> bool {
    let builders = workflow.role_names_for_purpose(ITERATION_BUILDER_PURPOSE);
    iteration
        .dev_ok_by
        .iter()
        .any(|approver| builders.iter().any(|r| r == approver))
}
