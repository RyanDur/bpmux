#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::backlog::{BacklogColumn, backlog_column, group_by_backlog_column};
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::{BoardState, Story, StoryId};
use chrono::{TimeZone, Utc};

fn story(num: u64, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(num),
        format!("story {num}"),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn iteration(id: u64, is_current: bool, story_ids: &[u64]) -> Iteration {
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
        complete: false,
    }
}

fn complete_but_still_current_iteration(id: u64, story_ids: &[u64]) -> Iteration {
    Iteration {
        complete: true,
        ..iteration(id, true, story_ids)
    }
}

#[test]
fn a_member_of_a_complete_but_still_current_iteration_is_not_current_work() {
    let iterations = [complete_but_still_current_iteration(1, &[10])];
    assert_eq!(
        backlog_column(&story(10, BoardState::InProgress), &iterations),
        Some(BacklogColumn::ReadyForWork),
        "after close and before the next designation, a complete iteration's members are planned work, not the current board — the canonical current-predicate excludes complete iterations"
    );
}

#[test]
fn a_stored_icebox_member_of_the_current_iteration_derives_ready_and_is_current() {
    let iterations = [iteration(1, true, &[10])];
    assert_eq!(
        backlog_column(&story(10, BoardState::NotStarted), &iterations),
        Some(BacklogColumn::Current),
        "design-doc divergence #3: membership in the current iteration IS the scheduling truth — a stored-Icebox member projects Ready and belongs to the current board"
    );
}

#[test]
fn a_board_position_story_in_the_current_iteration_is_current_work() {
    let iterations = [iteration(1, true, &[10, 11, 12])];
    for state in [
        BoardState::NotStarted,
        BoardState::InProgress,
        BoardState::Acceptance,
    ] {
        assert_eq!(
            backlog_column(&story(10, state.clone()), &iterations),
            Some(BacklogColumn::Current),
            "a {state:?} story in the current iteration IS the board — the current column"
        );
    }
}

#[test]
fn a_planned_story_in_a_non_current_iteration_is_ready_for_work() {
    let iterations = [iteration(1, true, &[10]), iteration(2, false, &[20])];
    assert_eq!(
        backlog_column(&story(20, BoardState::NotStarted), &iterations),
        Some(BacklogColumn::ReadyForWork),
        "planned work — in an iteration on the roadmap, but not the current one"
    );
}

#[test]
fn an_unplanned_story_is_icebox() {
    let iterations = [iteration(1, true, &[10])];
    assert_eq!(
        backlog_column(&story(99, BoardState::NotStarted), &iterations),
        Some(BacklogColumn::Icebox),
        "unplanned work — referenced by no iteration"
    );
}

#[test]
fn an_accepted_story_is_accepted_even_in_the_current_iteration() {
    let iterations = [iteration(1, true, &[10])];
    assert_eq!(
        backlog_column(&story(10, BoardState::Accepted), &iterations),
        Some(BacklogColumn::Accepted),
        "completed work — position wins over membership"
    );
}

#[test]
fn a_retired_story_is_archived_off_every_column() {
    let iterations = [iteration(1, true, &[10])];
    assert_eq!(
        backlog_column(&story(10, BoardState::Retired), &iterations),
        None,
        "archived work the owner does not need to see"
    );
}

#[test]
fn a_retired_story_is_archived_even_when_in_the_current_iteration() {
    let iterations = [iteration(1, true, &[10])];
    let retired = story(10, BoardState::Retired);
    assert_eq!(
        backlog_column(&retired, &iterations),
        None,
        "a retired story is archived even when it sits in the current iteration"
    );
}

#[test]
fn an_accepted_story_stays_in_the_accepted_column() {
    let accepted = story(10, BoardState::Accepted);
    assert_eq!(
        backlog_column(&accepted, &[]),
        Some(BacklogColumn::Accepted),
        "accepted = legitimate completion, not archival"
    );
}

#[test]
fn grouping_places_every_non_archived_story_in_exactly_one_column() {
    let iterations = [iteration(1, true, &[1, 2]), iteration(2, false, &[3])];
    let stories = vec![
        story(1, BoardState::NotStarted),
        story(2, BoardState::Accepted),
        story(3, BoardState::NotStarted),
        story(4, BoardState::NotStarted),
        story(5, BoardState::Retired),
    ];
    let grouped = group_by_backlog_column(&stories, &iterations);
    let placed: usize = grouped.values().map(Vec::len).sum();
    assert_eq!(
        placed, 4,
        "four visible stories; the retired one is archived"
    );
    let ids = |column: BacklogColumn| -> Vec<u64> {
        grouped
            .get(&column)
            .into_iter()
            .flatten()
            .map(|s| s.id.value())
            .collect()
    };
    assert_eq!(ids(BacklogColumn::Current), vec![1]);
    assert_eq!(ids(BacklogColumn::Accepted), vec![2]);
    assert_eq!(ids(BacklogColumn::ReadyForWork), vec![3]);
    assert_eq!(ids(BacklogColumn::Icebox), vec![4]);
}
