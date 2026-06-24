use serde::Serialize;

use crate::product_management::iteration::readiness::{
    IterationReadiness, building_iterations, iteration_readiness,
};
use crate::product_management::story::Story;
use crate::workflow::config::WorkflowConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConveneCheckState {
    Met,
    Gap,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConveneCheck {
    pub label: String,
    pub state: ConveneCheckState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DrillIterationState {
    Done,
    Ready,
    Next,
    Gap,
    NotStarted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DrillIteration {
    pub number: u64,
    pub name: String,
    pub order: u32,
    pub state: DrillIterationState,
    pub gap_count: u32,
    pub checks: Vec<ConveneCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConveneBanner {
    pub next_iteration_name: String,
    pub next_gap_count: u32,
    pub active_gap_label: Option<String>,
    pub then_iteration_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutcomeDrillIn {
    pub iterations: Vec<DrillIteration>,
    pub convene: Option<ConveneBanner>,
}

fn checks_in_order(r: &IterationReadiness) -> [(bool, &'static str); 4] {
    [
        (r.drafted, "drafted"),
        (r.stories_ordered, "stories-ordered"),
        (r.review_closed, "designer-review"),
        (r.dev_ok, "dev-OK"),
    ]
}

fn nothing_done(r: &IterationReadiness) -> bool {
    checks_in_order(r).iter().all(|(met, _)| !met)
}

fn checks_of(r: &IterationReadiness) -> Vec<ConveneCheck> {
    let mut active_gap_taken = false;
    checks_in_order(r)
        .iter()
        .map(|(met, label)| {
            let state = if *met {
                ConveneCheckState::Met
            } else if !active_gap_taken {
                active_gap_taken = true;
                ConveneCheckState::Gap
            } else {
                ConveneCheckState::Pending
            };
            ConveneCheck {
                label: (*label).to_string(),
                state,
            }
        })
        .collect()
}

fn first_unmet_label(r: &IterationReadiness) -> Option<String> {
    checks_in_order(r)
        .iter()
        .find(|(met, _)| !met)
        .map(|(_, label)| (*label).to_string())
}

fn readiness_of(iteration: &Story, all: &[Story], workflow: &WorkflowConfig) -> IterationReadiness {
    let members: Vec<&Story> = all
        .iter()
        .filter(|m| m.iteration_membership.map(|(it, _)| it) == Some(iteration.number))
        .collect();
    iteration_readiness(iteration, &members, workflow)
}

pub fn derive_outcome_drill_in(
    outcome_number: u64,
    stories: &[Story],
    workflow: &WorkflowConfig,
) -> OutcomeDrillIn {
    let mut building = building_iterations(outcome_number, stories);
    building.sort_by_key(|it| {
        (
            it.outcome_membership.map(|(_, p)| p).unwrap_or(0),
            it.number,
        )
    });

    let next_index = building.iter().position(|it| {
        !it.is_closed && {
            let r = readiness_of(it, stories, workflow);
            !nothing_done(&r) && r.unmet_count() > 0
        }
    });

    let iterations = building
        .iter()
        .enumerate()
        .map(|(index, it)| {
            let r = readiness_of(it, stories, workflow);
            let gap_count = r.unmet_count();
            let state = if it.is_closed {
                DrillIterationState::Done
            } else if nothing_done(&r) {
                DrillIterationState::NotStarted
            } else if gap_count == 0 {
                DrillIterationState::Ready
            } else if Some(index) == next_index {
                DrillIterationState::Next
            } else {
                DrillIterationState::Gap
            };
            let checks = match state {
                DrillIterationState::NotStarted | DrillIterationState::Done => Vec::new(),
                _ => checks_of(&r),
            };
            DrillIteration {
                number: it.number,
                name: it.title.clone(),
                order: index as u32 + 1,
                state,
                gap_count,
                checks,
            }
        })
        .collect();

    let convene = next_index.map(|ni| {
        let next = building[ni];
        let next_r = readiness_of(next, stories, workflow);
        ConveneBanner {
            next_iteration_name: next.title.clone(),
            next_gap_count: next_r.unmet_count(),
            active_gap_label: first_unmet_label(&next_r),
            then_iteration_name: building.get(ni + 1).map(|it| it.title.clone()),
        }
    });

    OutcomeDrillIn {
        iterations,
        convene,
    }
}
