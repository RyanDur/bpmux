use serde::Serialize;

use crate::product_management::board::domain::{Horizon, Story, StoryState, StoryType};
use crate::product_management::iteration::readiness::{building_iterations, outcome_readiness};
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

pub fn derive_roadmap_view(stories: &[Story], workflow: &WorkflowConfig) -> Vec<RoadmapBand> {
    BANDS
        .iter()
        .filter_map(|(horizon, label)| {
            let mut members: Vec<&Story> = stories
                .iter()
                .filter(|s| {
                    s.story_type == StoryType::Outcome
                        && s.horizon == Some(*horizon)
                        && s.state != StoryState::Retired
                })
                .collect();
            if members.is_empty() {
                return None;
            }
            members.sort_by_key(|s| (s.horizon_order.unwrap_or(u32::MAX), s.number));
            let outcomes: Vec<RoadmapOutcome> = members
                .iter()
                .map(|outcome| roadmap_outcome(outcome, stories, workflow))
                .collect();
            Some(RoadmapBand {
                horizon: *horizon,
                label: (*label).to_string(),
                outcomes,
            })
        })
        .collect()
}

fn roadmap_outcome(outcome: &Story, all: &[Story], workflow: &WorkflowConfig) -> RoadmapOutcome {
    let readiness = outcome_readiness(outcome, all, workflow);
    let building = building_iterations(outcome.number, all);
    let done = !building.is_empty() && building.iter().all(|it| it.is_closed);
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
        number: outcome.number,
        name: outcome.title.clone(),
        state,
        gap_count: readiness.gap_count,
    }
}
