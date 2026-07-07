#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::filter::*;
use super::selectors::{OutcomeCardTone, OutcomeState, OutcomeWhy, RoadmapOutcomeCard};
use crate::product_management::outcome::OutcomeId;
use crate::product_management::story::Horizon;

fn card(
    number: u64,
    name: &str,
    horizon: Horizon,
    state: OutcomeState,
    gap_count: u32,
) -> RoadmapOutcomeCard {
    RoadmapOutcomeCard {
        id: OutcomeId::new(number),
        name: name.to_string(),
        why: OutcomeWhy::default(),
        horizon,
        state,
        gap_count,
        tone: OutcomeCardTone::Now,
    }
}

#[test]
fn filter_by_state_keeps_only_matching_outcomes() {
    let cards = vec![
        card(1, "a", Horizon::Now, OutcomeState::Ready, 0),
        card(2, "b", Horizon::Now, OutcomeState::NeedsAttention, 3),
        card(3, "c", Horizon::Next, OutcomeState::Ready, 0),
    ];
    let filtered = filter_roadmap(
        cards,
        Some(OutcomeState::Ready),
        RoadmapOrder::Priority,
        None,
    );
    let numbers: Vec<u64> = filtered.outcomes.iter().map(|c| c.id.value()).collect();
    assert_eq!(numbers, vec![1, 3]);
    assert_eq!(filtered.total, 2);
    assert_eq!(filtered.shown, 2);
}

#[test]
fn priority_order_keeps_the_served_order_within_each_horizon() {
    let cards = vec![
        card(1, "a", Horizon::Now, OutcomeState::Ready, 0),
        card(2, "b", Horizon::Now, OutcomeState::NeedsAttention, 5),
        card(3, "c", Horizon::Next, OutcomeState::Ready, 0),
    ];
    let filtered = filter_roadmap(cards, None, RoadmapOrder::Priority, None);
    let numbers: Vec<u64> = filtered.outcomes.iter().map(|c| c.id.value()).collect();
    assert_eq!(numbers, vec![1, 2, 3]);
}

#[test]
fn gap_count_order_sorts_within_horizon_but_keeps_horizon_grouping() {
    let cards = vec![
        card(1, "a", Horizon::Now, OutcomeState::NeedsAttention, 1),
        card(2, "b", Horizon::Now, OutcomeState::NeedsAttention, 4),
        card(3, "c", Horizon::Next, OutcomeState::NeedsAttention, 9),
    ];
    let filtered = filter_roadmap(cards, None, RoadmapOrder::GapCount, None);
    let numbers: Vec<u64> = filtered.outcomes.iter().map(|c| c.id.value()).collect();
    assert_eq!(
        numbers,
        vec![2, 1, 3],
        "Now band sorted by gap-count desc, Next band stays after Now"
    );
}

#[test]
fn name_order_sorts_alphabetically_within_horizon() {
    let cards = vec![
        card(1, "zebra", Horizon::Now, OutcomeState::Ready, 0),
        card(2, "apple", Horizon::Now, OutcomeState::Ready, 0),
    ];
    let filtered = filter_roadmap(cards, None, RoadmapOrder::Name, None);
    let names: Vec<&str> = filtered.outcomes.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["apple", "zebra"]);
}

#[test]
fn limit_truncates_and_reports_total_and_shown() {
    let cards = vec![
        card(1, "a", Horizon::Now, OutcomeState::Ready, 0),
        card(2, "b", Horizon::Now, OutcomeState::Ready, 0),
        card(3, "c", Horizon::Next, OutcomeState::Ready, 0),
    ];
    let filtered = filter_roadmap(cards, None, RoadmapOrder::Priority, Some(2));
    assert_eq!(filtered.outcomes.len(), 2);
    assert_eq!(filtered.total, 3);
    assert_eq!(filtered.shown, 2);
}
