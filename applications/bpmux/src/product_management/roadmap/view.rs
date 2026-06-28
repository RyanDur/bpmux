use serde::Serialize;

use crate::product_management::iteration::readiness::{building_iterations, outcome_readiness};
use crate::product_management::outcome::Outcome;
use crate::product_management::story::{Horizon, Story};
use crate::workflow::config::WorkflowConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutcomeState {
    Done,
    Ready,
    NeedsAttention,
    Planned,
}

impl OutcomeState {
    pub const CLI_VALUES: &'static str = "done | ready | needs-attention | planned";

    pub fn from_cli(value: &str) -> Option<Self> {
        match value {
            "done" => Some(Self::Done),
            "ready" => Some(Self::Ready),
            "needs-attention" => Some(Self::NeedsAttention),
            "planned" => Some(Self::Planned),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoadmapOutcome {
    pub number: u64,
    pub name: String,
    pub state: OutcomeState,
    pub gap_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoadmapBand {
    pub horizon: Horizon,
    pub label: String,
    pub outcomes: Vec<RoadmapOutcome>,
}

const BANDS: [(Horizon, &str); 3] = [
    (Horizon::Now, "Now"),
    (Horizon::Next, "Next"),
    (Horizon::Later, "Later"),
];

pub fn derive_roadmap_view(
    outcomes: &[Outcome],
    all_stories: &[Story],
    workflow: &WorkflowConfig,
) -> Vec<RoadmapBand> {
    BANDS
        .iter()
        .filter_map(|(horizon, label)| {
            let mut members: Vec<&Outcome> = outcomes
                .iter()
                .filter(|outcome| outcome.horizon() == Some(*horizon) && !outcome.is_retired())
                .collect();
            if members.is_empty() {
                return None;
            }
            members.sort_by_key(|outcome| {
                (
                    outcome.horizon_order().unwrap_or(u32::MAX),
                    outcome.id().saved_number().unwrap_or(u64::MAX),
                )
            });
            let outcomes: Vec<RoadmapOutcome> = members
                .iter()
                .map(|outcome| roadmap_outcome(outcome, all_stories, workflow))
                .collect();
            Some(RoadmapBand {
                horizon: *horizon,
                label: (*label).to_string(),
                outcomes,
            })
        })
        .collect()
}

fn roadmap_outcome(outcome: &Outcome, all: &[Story], workflow: &WorkflowConfig) -> RoadmapOutcome {
    let number = outcome.id().saved_number().unwrap_or(0);
    let readiness = outcome_readiness(number, all, workflow);
    let building = building_iterations(number, all);
    let done = !building.is_empty() && building.iter().all(|it| it.complete);
    let state = if done {
        OutcomeState::Done
    } else if readiness.iteration_count == 0 {
        OutcomeState::Planned
    } else if readiness.all_ready {
        OutcomeState::Ready
    } else {
        OutcomeState::NeedsAttention
    };
    RoadmapOutcome {
        number,
        name: outcome.content.title.clone(),
        state,
        gap_count: readiness.gap_count,
    }
}
