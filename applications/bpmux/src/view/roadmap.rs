use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::product_management::board::RefreshDriver;
use crate::product_management::iteration::readiness::{
    OutcomeReadiness, building_iterations, outcome_readiness,
};
use crate::product_management::story::Horizon;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OutcomeWhy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benefit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeCardTone {
    Now,
    Ready,
    Gap,
    Future,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapOutcomeInput {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub horizon: Option<Horizon>,
    pub horizon_order: Option<u32>,
    pub done: bool,
    pub readiness: Option<OutcomeReadiness>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadmapOutcomeCard {
    pub number: u64,
    pub name: String,
    pub why: OutcomeWhy,
    pub horizon: Horizon,
    pub state: OutcomeState,
    pub gap_count: u32,
    pub tone: OutcomeCardTone,
}

pub fn roadmap_cards(refresh: &Arc<dyn RefreshDriver>) -> Vec<RoadmapOutcomeCard> {
    derive_roadmap(&roadmap_inputs(refresh))
}

fn roadmap_inputs(refresh: &Arc<dyn RefreshDriver>) -> Vec<RoadmapOutcomeInput> {
    let all_iterations = refresh.iterations();
    let workflow = crate::workflow::current_workflow();
    refresh
        .outcomes()
        .iter()
        .filter(|outcome| !outcome.is_retired())
        .map(|outcome| {
            let number = outcome.id().saved_id().map(|id| id.value()).unwrap_or(0);
            let building = building_iterations(outcome, &all_iterations);
            let done = !building.is_empty() && building.iter().all(|iteration| iteration.complete);
            RoadmapOutcomeInput {
                number,
                title: outcome.content.title.clone(),
                body: outcome.content.body.clone().unwrap_or_default(),
                horizon: outcome.horizon(),
                horizon_order: outcome.horizon_order(),
                done,
                readiness: Some(outcome_readiness(outcome, &all_iterations, workflow)),
            }
        })
        .collect()
}

pub fn derive_roadmap(outcomes: &[RoadmapOutcomeInput]) -> Vec<RoadmapOutcomeCard> {
    let mut ordered: Vec<&RoadmapOutcomeInput> = outcomes.iter().collect();
    ordered.sort_by_key(|outcome| {
        (
            horizon_rank(outcome.horizon.unwrap_or(Horizon::Later)),
            outcome.horizon_order.unwrap_or(u32::MAX),
            outcome.number,
        )
    });
    ordered.into_iter().map(to_card).collect()
}

pub(crate) fn horizon_rank(horizon: Horizon) -> u8 {
    match horizon {
        Horizon::Now => 0,
        Horizon::Next => 1,
        Horizon::Later => 2,
    }
}

fn to_card(outcome: &RoadmapOutcomeInput) -> RoadmapOutcomeCard {
    let horizon = outcome.horizon.unwrap_or(Horizon::Later);
    let state = state_of(outcome.done, outcome.readiness);
    RoadmapOutcomeCard {
        number: outcome.number,
        name: outcome.title.clone(),
        why: parse_outcome_why(&outcome.body),
        horizon,
        state,
        gap_count: outcome.readiness.map(|r| r.gap_count).unwrap_or(0),
        tone: tone_of(horizon, state),
    }
}

fn state_of(done: bool, readiness: Option<OutcomeReadiness>) -> OutcomeState {
    if done {
        return OutcomeState::Done;
    }
    match readiness {
        Some(r) if r.iteration_count == 0 => OutcomeState::Planned,
        Some(r) if r.all_ready => OutcomeState::Ready,
        Some(_) => OutcomeState::NeedsAttention,
        None => OutcomeState::Planned,
    }
}

fn tone_of(horizon: Horizon, state: OutcomeState) -> OutcomeCardTone {
    match state {
        OutcomeState::NeedsAttention => OutcomeCardTone::Gap,
        OutcomeState::Done | OutcomeState::Ready => OutcomeCardTone::Ready,
        OutcomeState::Planned => {
            if horizon == Horizon::Now {
                OutcomeCardTone::Now
            } else {
                OutcomeCardTone::Future
            }
        }
    }
}

pub fn parse_outcome_why(body: &str) -> OutcomeWhy {
    let mut why = OutcomeWhy::default();
    for raw_line in body.lines() {
        let stripped = raw_line.replace("**", "");
        let line = stripped.trim();
        let Some(colon) = line.find(':') else {
            continue;
        };
        let label = line[..colon].trim().to_lowercase();
        let value = line[colon + 1..].trim().to_string();
        match label.as_str() {
            "benefit" if why.benefit.is_none() => why.benefit = Some(value),
            "success-metric" | "metric" if why.metric.is_none() => why.metric = Some(value),
            "confidence" if why.confidence.is_none() => why.confidence = Some(value),
            "value" if why.value.is_none() => why.value = Some(value),
            _ => {}
        }
    }
    why
}
