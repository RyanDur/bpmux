use std::cmp::Ordering;

use super::selectors::{OutcomeState, RoadmapOutcomeCard, horizon_rank};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadmapOrder {
    Priority,
    GapCount,
    State,
    Name,
}

impl RoadmapOrder {
    pub const CLI_VALUES: &'static str = "priority | gap-count | state | name";

    pub fn from_cli(value: &str) -> Option<Self> {
        match value {
            "priority" => Some(Self::Priority),
            "gap-count" => Some(Self::GapCount),
            "state" => Some(Self::State),
            "name" => Some(Self::Name),
            _ => None,
        }
    }
}

pub struct FilteredRoadmap {
    pub outcomes: Vec<RoadmapOutcomeCard>,
    pub total: usize,
    pub shown: usize,
}

pub fn filter_roadmap(
    cards: Vec<RoadmapOutcomeCard>,
    state: Option<OutcomeState>,
    order: RoadmapOrder,
    limit: Option<usize>,
) -> FilteredRoadmap {
    let mut outcomes: Vec<RoadmapOutcomeCard> = cards
        .into_iter()
        .filter(|card| state.is_none_or(|wanted| card.state == wanted))
        .collect();
    outcomes.sort_by(|a, b| {
        horizon_rank(a.horizon)
            .cmp(&horizon_rank(b.horizon))
            .then_with(|| order_within_horizon(a, b, order))
    });
    let total = outcomes.len();
    let shown = match limit {
        Some(limit) => {
            outcomes.truncate(limit);
            total.min(limit)
        }
        None => total,
    };
    FilteredRoadmap {
        outcomes,
        total,
        shown,
    }
}

fn order_within_horizon(
    a: &RoadmapOutcomeCard,
    b: &RoadmapOutcomeCard,
    order: RoadmapOrder,
) -> Ordering {
    match order {
        RoadmapOrder::Priority => Ordering::Equal,
        RoadmapOrder::GapCount => b.gap_count.cmp(&a.gap_count).then(a.id.cmp(&b.id)),
        RoadmapOrder::State => state_rank(a.state)
            .cmp(&state_rank(b.state))
            .then(a.id.cmp(&b.id)),
        RoadmapOrder::Name => a.name.cmp(&b.name).then(a.id.cmp(&b.id)),
    }
}

fn state_rank(state: OutcomeState) -> u8 {
    match state {
        OutcomeState::NeedsAttention => 0,
        OutcomeState::Ready => 1,
        OutcomeState::Planned => 2,
        OutcomeState::Done => 3,
    }
}
