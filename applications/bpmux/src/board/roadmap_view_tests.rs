#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::roadmap_view::*;
use crate::board::domain::{Horizon, Story, StoryState, StoryType};
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
    s.horizon = Some(horizon);
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
    s.outcome_membership = Some((outcome_number, position));
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
    first.horizon_order = Some(1);
    let mut second = outcome(11, "A", Horizon::Now);
    second.horizon_order = Some(2);
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
    feature.horizon = Some(Horizon::Now);
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
    it.iteration_review_closed_by = vec!["designer".to_string()];
    it.iteration_dev_ok_by = vec!["dev".to_string()];
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
