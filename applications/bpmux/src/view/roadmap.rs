use serde::{Deserialize, Serialize};

use crate::product_management::iteration::readiness::OutcomeReadiness;
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
#[serde(rename_all = "lowercase")]
pub enum RoadmapReadinessState {
    Ready,
    Gap,
    None,
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
    pub readiness: Option<OutcomeReadiness>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadmapOutcomeCard {
    pub number: u64,
    pub name: String,
    pub why: OutcomeWhy,
    pub horizon: Horizon,
    pub readiness_state: RoadmapReadinessState,
    pub gap_count: u32,
    pub tone: OutcomeCardTone,
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

fn horizon_rank(horizon: Horizon) -> u8 {
    match horizon {
        Horizon::Now => 0,
        Horizon::Next => 1,
        Horizon::Later => 2,
    }
}

fn to_card(outcome: &RoadmapOutcomeInput) -> RoadmapOutcomeCard {
    let horizon = outcome.horizon.unwrap_or(Horizon::Later);
    let readiness_state = readiness_state_of(outcome.readiness);
    RoadmapOutcomeCard {
        number: outcome.number,
        name: outcome.title.clone(),
        why: parse_outcome_why(&outcome.body),
        horizon,
        readiness_state,
        gap_count: outcome.readiness.map(|r| r.gap_count).unwrap_or(0),
        tone: tone_of(horizon, readiness_state),
    }
}

fn readiness_state_of(readiness: Option<OutcomeReadiness>) -> RoadmapReadinessState {
    match readiness {
        None => RoadmapReadinessState::None,
        Some(r) if r.iteration_count == 0 => RoadmapReadinessState::None,
        Some(r) if r.all_ready => RoadmapReadinessState::Ready,
        Some(_) => RoadmapReadinessState::Gap,
    }
}

fn tone_of(horizon: Horizon, state: RoadmapReadinessState) -> OutcomeCardTone {
    match state {
        RoadmapReadinessState::Gap => OutcomeCardTone::Gap,
        RoadmapReadinessState::Ready => OutcomeCardTone::Ready,
        RoadmapReadinessState::None => {
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

pub fn render_roadmap(cards: &[RoadmapOutcomeCard]) -> String {
    let mut out = String::from("ROADMAP\n");
    if cards.is_empty() {
        out.push_str("  (no outcomes yet)\n");
        return out;
    }
    let mut current_horizon: Option<Horizon> = None;
    for card in cards {
        let band = if current_horizon == Some(card.horizon) {
            ""
        } else {
            current_horizon = Some(card.horizon);
            card.horizon.band_label()
        };
        out.push_str(&format!(
            "{band:<6} #{}  {}  {}\n",
            card.number,
            card.name,
            state_text(card)
        ));
    }
    out
}

fn state_text(card: &RoadmapOutcomeCard) -> String {
    match card.readiness_state {
        RoadmapReadinessState::Ready => "ready".to_string(),
        RoadmapReadinessState::Gap => {
            let count = card.gap_count;
            format!("{count} gap{}", if count == 1 { "" } else { "s" })
        }
        RoadmapReadinessState::None => "planned".to_string(),
    }
}
