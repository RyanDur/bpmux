use serde::Serialize;

use crate::product_management::board::adapters::github::{
    iteration_dev_ok_by_from_labels, iteration_review_closers_from_labels,
};
use crate::product_management::story::Story;
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

pub fn building_iterations(outcome_number: u64, all: &[Story]) -> Vec<&Story> {
    all.iter()
        .filter(|s| {
            s.story_type == crate::product_management::story::StoryType::Iteration
                && crate::product_management::board::adapters::github::outcome_membership_from_labels(
                    &s.raw_labels,
                )
                .map(|(o, _)| o)
                    == Some(outcome_number)
        })
        .collect()
}

pub fn outcome_readiness(
    outcome: &Story,
    all: &[Story],
    workflow: &WorkflowConfig,
) -> OutcomeReadiness {
    let building = building_iterations(outcome.number, all);
    let gap_count = building
        .iter()
        .map(|iteration| {
            let members: Vec<&Story> = all
                .iter()
                .filter(|m| m.iteration_membership.map(|(it, _)| it) == Some(iteration.number))
                .collect();
            iteration_readiness(iteration, &members, workflow).unmet_count()
        })
        .sum();
    let iteration_count = building.len() as u32;
    OutcomeReadiness {
        iteration_count,
        gap_count,
        all_ready: iteration_count > 0 && gap_count == 0,
    }
}

pub fn iteration_readiness(
    iteration: &Story,
    members: &[&Story],
    workflow: &WorkflowConfig,
) -> IterationReadiness {
    IterationReadiness {
        drafted: drafted(iteration),
        stories_ordered: stories_ordered(members),
        review_closed: review_closed(iteration, workflow),
        dev_ok: dev_ok(iteration, workflow),
    }
}

fn drafted(iteration: &Story) -> bool {
    iteration
        .body
        .as_deref()
        .is_some_and(|b| !b.trim().is_empty())
}

fn stories_ordered(members: &[&Story]) -> bool {
    if members.is_empty() {
        return false;
    }
    let positions: Vec<u32> = members
        .iter()
        .filter_map(|s| s.iteration_membership.map(|(_, position)| position))
        .collect();
    if positions.len() != members.len() {
        return false;
    }
    let mut distinct = positions.clone();
    distinct.sort_unstable();
    distinct.dedup();
    distinct.len() == positions.len()
}

fn review_closed(iteration: &Story, workflow: &WorkflowConfig) -> bool {
    let reviewers = workflow.role_ids_for_purpose(ITERATION_REVIEWER_PURPOSE);
    iteration_review_closers_from_labels(&iteration.raw_labels)
        .iter()
        .any(|closer| reviewers.iter().any(|r| r == closer))
}

fn dev_ok(iteration: &Story, workflow: &WorkflowConfig) -> bool {
    let builders = workflow.role_ids_for_purpose(ITERATION_BUILDER_PURPOSE);
    iteration_dev_ok_by_from_labels(&iteration.raw_labels)
        .iter()
        .any(|approver| builders.iter().any(|r| r == approver))
}
