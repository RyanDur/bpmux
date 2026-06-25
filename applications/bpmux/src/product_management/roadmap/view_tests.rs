#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view::*;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::story::{Horizon, Story, StoryState, StoryType};
use crate::workflow::bundled_default::bundled_default;
use chrono::Utc;

fn outcome(number: u64, title: &str, horizon: Horizon) -> Story {
    let mut s = Story::new_minimal(
        number,
        title.to_string(),
        StoryState::Icebox,
        Utc::now(),
        "pm".to_string(),
    );
    s.story_type = StoryType::Outcome;
    s.raw_labels.push(GhLabelCodec.encode_horizon(horizon));
    s
}

fn iteration_for(number: u64, outcome_number: u64, position: u32) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration #{number}"),
        StoryState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    s.story_type = StoryType::Iteration;
    s.raw_labels
        .push(format!("parent-outcome-#{outcome_number}.{position}"));
    s.body = Some("## composed".to_string());
    s
}

#[test]
fn outcomes_group_into_now_next_later_priority_bands() {
    let stories = vec![
        outcome(10, "Later thing", Horizon::Later),
        outcome(11, "Now thing", Horizon::Now),
        outcome(12, "Next thing", Horizon::Next),
    ];
    let bands = derive_roadmap_view(&stories, &bundled_default());
    let labels: Vec<&str> = bands.iter().map(|b| b.label.as_str()).collect();
    assert_eq!(
        labels,
        vec!["Now", "Next", "Later"],
        "bands render in Now/Next/Later priority order (owner-parity)"
    );
}

#[test]
fn a_retired_outcome_drops_from_the_roadmap_view_1761() {
    let live = outcome(11, "Live outcome", Horizon::Now);
    let mut retired = outcome(12, "Abandoned outcome", Horizon::Now);
    retired.state = StoryState::Retired;
    let bands = derive_roadmap_view(&[live, retired], &bundled_default());
    let numbers: Vec<u64> = bands
        .iter()
        .flat_map(|b| &b.outcomes)
        .map(|o| o.number)
        .collect();
    assert_eq!(
        numbers,
        vec![11],
        "a retired outcome is filtered on its domain-state at edge-in (the #1234-class: NOT rendered from raw is_closed); only the live outcome appears"
    );
}

#[test]
fn a_band_with_only_retired_outcomes_is_omitted_1761() {
    let mut retired = outcome(12, "Abandoned", Horizon::Now);
    retired.state = StoryState::Retired;
    let bands = derive_roadmap_view(&[retired], &bundled_default());
    assert!(
        bands.is_empty(),
        "a band whose only outcome is retired renders nothing — retire is done only when the view drops it"
    );
}

#[test]
fn a_horizon_band_with_no_outcomes_is_omitted() {
    let stories = vec![outcome(11, "Now thing", Horizon::Now)];
    let bands = derive_roadmap_view(&stories, &bundled_default());
    assert_eq!(bands.len(), 1, "only the populated band renders");
    assert_eq!(bands[0].horizon, Horizon::Now);
}

#[test]
fn outcomes_within_a_band_sort_by_number() {
    let stories = vec![
        outcome(13, "B", Horizon::Now),
        outcome(11, "A", Horizon::Now),
    ];
    let bands = derive_roadmap_view(&stories, &bundled_default());
    let numbers: Vec<u64> = bands[0].outcomes.iter().map(|o| o.number).collect();
    assert_eq!(numbers, vec![11, 13]);
}

#[test]
fn an_explicit_within_band_position_overrides_number_order() {
    let mut first = outcome(13, "B", Horizon::Now);
    first.raw_labels.push(GhLabelCodec.encode_horizon_order(1));
    let mut second = outcome(11, "A", Horizon::Now);
    second.raw_labels.push(GhLabelCodec.encode_horizon_order(2));
    let unpositioned = outcome(12, "C", Horizon::Now);
    let bands = derive_roadmap_view(&[second, unpositioned, first], &bundled_default());
    let numbers: Vec<u64> = bands[0].outcomes.iter().map(|o| o.number).collect();
    assert_eq!(
        numbers,
        vec![13, 11, 12],
        "positioned outcomes lead in explicit order; an unpositioned outcome trails by number"
    );
}

#[test]
fn a_non_outcome_story_is_never_a_roadmap_row_even_with_a_horizon() {
    let mut feature = Story::new_minimal(
        99,
        "a feature".to_string(),
        StoryState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    feature
        .raw_labels
        .push(GhLabelCodec.encode_horizon(Horizon::Now));
    let stories = vec![outcome(11, "Now thing", Horizon::Now), feature];
    let bands = derive_roadmap_view(&stories, &bundled_default());
    assert_eq!(bands[0].outcomes.len(), 1, "only the outcome is a row");
    assert_eq!(bands[0].outcomes[0].number, 11);
}

#[test]
fn an_outcome_with_no_iterations_reads_planned() {
    let stories = vec![outcome(11, "Now thing", Horizon::Now)];
    let bands = derive_roadmap_view(&stories, &bundled_default());
    assert_eq!(bands[0].outcomes[0].state, OutcomeState::Planned);
}

#[test]
fn an_outcome_with_an_unready_iteration_reads_needs_attention_with_gap_count() {
    let stories = vec![
        outcome(11, "Now thing", Horizon::Now),
        iteration_for(20, 11, 1),
    ];
    let bands = derive_roadmap_view(&stories, &bundled_default());
    let o = &bands[0].outcomes[0];
    assert_eq!(o.state, OutcomeState::NeedsAttention);
    assert_eq!(
        o.gap_count, 3,
        "drafted met; stories-ordered + review + dev-ok unmet"
    );
}

#[test]
fn an_outcome_whose_iteration_meets_every_check_reads_ready() {
    let mut it = iteration_for(20, 11, 1);
    it.raw_labels
        .push(GhLabelCodec.encode_iteration_review_closed("designer"));
    it.raw_labels
        .push(GhLabelCodec.encode_iteration_dev_ok("dev"));
    let mut member = Story::new_minimal(
        30,
        "member".to_string(),
        StoryState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    member.iteration_membership = Some((20, 1));
    let stories = vec![outcome(11, "Now thing", Horizon::Now), it, member];
    let bands = derive_roadmap_view(&stories, &bundled_default());
    let o = &bands[0].outcomes[0];
    assert_eq!(o.state, OutcomeState::Ready);
    assert_eq!(o.gap_count, 0);
}

#[test]
fn an_outcome_whose_building_iterations_are_all_closed_reads_done() {
    let mut it = iteration_for(20, 11, 1);
    it.is_closed = true;
    let stories = vec![outcome(11, "Now thing", Horizon::Now), it];
    let bands = derive_roadmap_view(&stories, &bundled_default());
    assert_eq!(
        bands[0].outcomes[0].state,
        OutcomeState::Done,
        "all building iterations closed → outcome done, dominating unmet readiness checks"
    );
}
