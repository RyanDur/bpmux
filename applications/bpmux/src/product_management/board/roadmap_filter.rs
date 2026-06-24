use crate::product_management::board::roadmap_view::{OutcomeState, RoadmapBand};

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

pub struct RoadmapQuery {
    pub state: Option<OutcomeState>,
    pub order: RoadmapOrder,
    pub limit: Option<usize>,
}

pub struct FilteredRoadmap {
    pub bands: Vec<RoadmapBand>,
    pub total: usize,
    pub shown: usize,
}

impl FilteredRoadmap {
    pub fn elided(&self) -> usize {
        self.total.saturating_sub(self.shown)
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

fn order_within_band(band: &mut RoadmapBand, order: RoadmapOrder) {
    match order {
        RoadmapOrder::Priority => band.outcomes.sort_by_key(|o| o.number),
        RoadmapOrder::GapCount => band
            .outcomes
            .sort_by(|a, b| b.gap_count.cmp(&a.gap_count).then(a.number.cmp(&b.number))),
        RoadmapOrder::State => band.outcomes.sort_by(|a, b| {
            state_rank(a.state)
                .cmp(&state_rank(b.state))
                .then(a.number.cmp(&b.number))
        }),
        RoadmapOrder::Name => band
            .outcomes
            .sort_by(|a, b| a.name.cmp(&b.name).then(a.number.cmp(&b.number))),
    }
}

pub fn apply_roadmap_query(bands: Vec<RoadmapBand>, query: &RoadmapQuery) -> FilteredRoadmap {
    let mut narrowed: Vec<RoadmapBand> = bands
        .into_iter()
        .map(|mut band| {
            if let Some(state) = query.state {
                band.outcomes.retain(|o| o.state == state);
            }
            order_within_band(&mut band, query.order);
            band
        })
        .filter(|band| !band.outcomes.is_empty())
        .collect();

    let total: usize = narrowed.iter().map(|b| b.outcomes.len()).sum();

    let shown = match query.limit {
        Some(limit) => {
            let mut remaining = limit;
            for band in &mut narrowed {
                if remaining == 0 {
                    band.outcomes.clear();
                } else if band.outcomes.len() > remaining {
                    band.outcomes.truncate(remaining);
                    remaining = 0;
                } else {
                    remaining -= band.outcomes.len();
                }
            }
            narrowed.retain(|b| !b.outcomes.is_empty());
            total.min(limit)
        }
        None => total,
    };

    FilteredRoadmap {
        bands: narrowed,
        total,
        shown,
    }
}
