#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::filter::*;
use super::view::{OutcomeState, RoadmapBand, RoadmapOutcome};
use crate::product_management::story::Horizon;

fn outcome(number: u64, name: &str, state: OutcomeState, gap_count: u32) -> RoadmapOutcome {
    RoadmapOutcome {
        number,
        name: name.to_string(),
        state,
        gap_count,
    }
}

fn band(horizon: Horizon, label: &str, outcomes: Vec<RoadmapOutcome>) -> RoadmapBand {
    RoadmapBand {
        horizon,
        label: label.to_string(),
        outcomes,
    }
}

fn sample() -> Vec<RoadmapBand> {
    vec![
        band(
            Horizon::Now,
            "Now",
            vec![
                outcome(10, "Charlie", OutcomeState::Ready, 0),
                outcome(11, "Alpha", OutcomeState::NeedsAttention, 3),
            ],
        ),
        band(
            Horizon::Next,
            "Next",
            vec![
                outcome(12, "Bravo", OutcomeState::NeedsAttention, 1),
                outcome(13, "Delta", OutcomeState::Planned, 0),
            ],
        ),
    ]
}

fn query(state: Option<OutcomeState>, order: RoadmapOrder, limit: Option<usize>) -> RoadmapQuery {
    RoadmapQuery {
        state,
        order,
        limit,
    }
}

#[test]
fn state_filter_keeps_only_matching_outcomes_and_drops_emptied_bands() {
    let result = apply_roadmap_query(
        sample(),
        &query(
            Some(OutcomeState::NeedsAttention),
            RoadmapOrder::Priority,
            None,
        ),
    );
    let numbers: Vec<u64> = result
        .bands
        .iter()
        .flat_map(|b| b.outcomes.iter().map(|o| o.number))
        .collect();
    assert_eq!(
        numbers,
        vec![11, 12],
        "only needs-attention outcomes remain"
    );
    assert_eq!(result.total, 2);
    assert_eq!(result.elided(), 0);
}

#[test]
fn priority_order_is_band_then_number_owner_parity() {
    let result = apply_roadmap_query(sample(), &query(None, RoadmapOrder::Priority, None));
    assert_eq!(result.bands[0].label, "Now");
    assert_eq!(
        result.bands[0]
            .outcomes
            .iter()
            .map(|o| o.number)
            .collect::<Vec<_>>(),
        vec![10, 11],
        "within a band, priority order is by number (the substrate order the owner sees)"
    );
}

#[test]
fn gap_count_order_sorts_most_attention_first_within_band() {
    let result = apply_roadmap_query(sample(), &query(None, RoadmapOrder::GapCount, None));
    assert_eq!(
        result.bands[0]
            .outcomes
            .iter()
            .map(|o| o.number)
            .collect::<Vec<_>>(),
        vec![11, 10],
        "Alpha (3 gaps) sorts ahead of Charlie (0 gaps) in the Now band"
    );
}

#[test]
fn name_order_sorts_alphabetically_within_band() {
    let result = apply_roadmap_query(sample(), &query(None, RoadmapOrder::Name, None));
    assert_eq!(
        result.bands[0]
            .outcomes
            .iter()
            .map(|o| o.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Alpha", "Charlie"]
    );
}

#[test]
fn limit_caps_total_rows_across_bands_and_reports_the_elided_count() {
    let result = apply_roadmap_query(sample(), &query(None, RoadmapOrder::Priority, Some(3)));
    let shown: usize = result.bands.iter().map(|b| b.outcomes.len()).sum();
    assert_eq!(shown, 3, "the first 3 outcomes across bands in order");
    assert_eq!(result.total, 4, "total before the limit");
    assert_eq!(result.shown, 3);
    assert_eq!(
        result.elided(),
        1,
        "1 of 4 elided — labelled, never silently truncated"
    );
}

#[test]
fn filters_compose_state_then_order_then_limit() {
    let result = apply_roadmap_query(
        sample(),
        &query(
            Some(OutcomeState::NeedsAttention),
            RoadmapOrder::GapCount,
            Some(1),
        ),
    );
    let numbers: Vec<u64> = result
        .bands
        .iter()
        .flat_map(|b| b.outcomes.iter().map(|o| o.number))
        .collect();
    assert_eq!(
        numbers,
        vec![11],
        "needs-attention, gap-count-first, top 1 = Alpha (3 gaps)"
    );
    assert_eq!(
        result.total, 2,
        "two matched needs-attention before the limit"
    );
    assert_eq!(result.elided(), 1);
}
