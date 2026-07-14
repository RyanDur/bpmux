#![allow(clippy::unwrap_used, clippy::expect_used)]

use board::{Column, column_of};
use chrono::Utc;
use product_management::iteration::{Iteration, IterationId};
use product_management::story::{Stage, Story, StoryId, Work};
use source_codec::mapper::state_from_labels;

fn story_in(state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(1),
        "sample".to_string(),
        state,
        Utc::now(),
        "pm".to_string(),
    )
}

fn current_iteration_of(members: &[u64]) -> Vec<Iteration> {
    vec![Iteration {
        id: IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc::now(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: members.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }]
}

#[test]
fn stacked_rejected_needs_fixes_ready_for_accept_decodes_to_not_started_rework() {
    let stacked = vec![
        "ready for accept".to_string(),
        "needs fixes".to_string(),
        "rejected".to_string(),
    ];
    assert_eq!(
        state_from_labels(&stacked, false),
        Work::NotStarted,
        "the rejected rework-signal wins the stacked-label priority and lands not-started; the projection supplies scheduling from membership"
    );
}

#[test]
fn stacked_accepted_rejected_decodes_accepted() {
    let stacked = vec!["accepted".to_string(), "rejected".to_string()];
    assert_eq!(state_from_labels(&stacked, false), Work::Accepted);
}

#[test]
fn stacked_needs_fixes_in_progress_decodes_in_progress() {
    let stacked = vec!["in progress".to_string(), "needs fixes".to_string()];
    assert_eq!(state_from_labels(&stacked, false), Work::InProgress);
}

#[test]
fn stacked_ready_in_progress_decodes_in_progress() {
    let stacked = vec!["ready".to_string(), "in progress".to_string()];
    assert_eq!(
        state_from_labels(&stacked, false),
        Work::InProgress,
        "a stale outbound ready mirror-label never outranks a real work-state label"
    );
}

#[test]
fn stacked_delivered_ready_for_accept_decodes_to_acceptance() {
    let stacked = vec!["delivered".to_string(), "ready for accept".to_string()];
    assert_eq!(state_from_labels(&stacked, false), Work::Acceptance);
}

#[test]
fn a_not_started_member_of_the_current_iteration_buckets_to_the_ready_column() {
    assert_eq!(
        column_of(
            &story_in(Work::NotStarted),
            None,
            &current_iteration_of(&[1])
        ),
        Column::Ready
    );
}

#[test]
fn in_progress_state_buckets_to_in_progress_column() {
    assert_eq!(
        column_of(&story_in(Work::InProgress), None, &[]),
        Column::Middle(Stage::Dev.id())
    );
}

#[test]
fn retired_state_buckets_to_retired_column() {
    assert_eq!(
        column_of(&story_in(Work::Retired), None, &[]),
        Column::Retired,
        "concluded work lands in the RETIRED column so the ACCEPTED success-narrative stays clean"
    );
}

#[test]
fn empty_labels_open_decodes_not_started() {
    assert_eq!(state_from_labels(&[], false), Work::NotStarted);
}

#[test]
fn empty_labels_closed_decodes_accepted() {
    assert_eq!(state_from_labels(&[], true), Work::Accepted);
}

#[test]
fn case_insensitive_label_matching() {
    let stacked = vec!["REJECTED".to_string(), "Ready For Accept".to_string()];
    assert_eq!(state_from_labels(&stacked, false), Work::NotStarted);
}
