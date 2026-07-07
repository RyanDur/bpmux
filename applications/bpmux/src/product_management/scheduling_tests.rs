#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::iteration::{Iteration, IterationId};
use super::scheduling::{ProjectedState, projected_state};
use super::story::{Story, StoryId, Work};
use chrono::{TimeZone, Utc};

fn story(num: u64, state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(num),
        format!("story {num}"),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn iteration(id: u64, is_current: bool, complete: bool, story_ids: &[u64]) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: story_ids.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current,
        complete,
    }
}

#[test]
fn a_not_started_member_of_the_current_iteration_projects_ready() {
    let iterations = [iteration(1, true, false, &[10])];
    assert_eq!(
        projected_state(&story(10, Work::NotStarted), &iterations),
        ProjectedState::Ready
    );
}

#[test]
fn a_not_started_story_outside_the_current_iteration_projects_icebox() {
    let iterations = [iteration(1, true, false, &[10])];
    assert_eq!(
        projected_state(&story(99, Work::NotStarted), &iterations),
        ProjectedState::Icebox
    );
}

#[test]
fn a_story_in_a_future_iteration_projects_icebox() {
    let iterations = [
        iteration(1, true, false, &[10]),
        iteration(2, false, false, &[20]),
    ];
    assert_eq!(
        projected_state(&story(20, Work::NotStarted), &iterations),
        ProjectedState::Icebox
    );
}

#[test]
fn a_non_not_started_state_passes_through_untouched() {
    let iterations = [iteration(1, true, false, &[10])];
    for (state, projected) in [
        (Work::InProgress, ProjectedState::InProgress),
        (Work::Acceptance, ProjectedState::Acceptance),
        (Work::Accepted, ProjectedState::Accepted),
        (Work::Retired, ProjectedState::Retired),
    ] {
        assert_eq!(
            projected_state(&story(10, state.clone()), &iterations),
            projected
        );
    }
}

#[test]
fn with_no_current_iteration_designated_nothing_projects_ready() {
    let iterations = [iteration(2, false, false, &[10])];
    assert_eq!(
        projected_state(&story(10, Work::NotStarted), &iterations),
        ProjectedState::Icebox
    );
}

#[test]
fn a_complete_but_still_current_iteration_does_not_count_as_current() {
    let iterations = [iteration(1, true, true, &[10])];
    assert_eq!(
        projected_state(&story(10, Work::NotStarted), &iterations),
        ProjectedState::Icebox
    );
}
