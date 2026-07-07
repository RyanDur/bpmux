#![allow(clippy::unwrap_used, clippy::expect_used)]

use bpmux::integrations::github::mapper::state_from_labels;
use bpmux::product_management::story::{BoardState, Stage, Story, StoryId};
use bpmux::view::board::arrangement::{Column, column_of};
use chrono::Utc;

fn story_in(state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(1),
        "sample".to_string(),
        state,
        Utc::now(),
        "pm".to_string(),
    )
}

#[test]
fn stacked_rejected_needs_fixes_ready_for_accept_decodes_to_ready() {
    // #1146 slice 3: stale "rejected" label projects to Ready (where
    // reject_story now transitions per slice 1). Pre-collapse stories
    // with stacked workflow-state labels still decode to a sensible
    // current-state until the next reap.
    let stacked = vec![
        "ready for accept".to_string(),
        "needs fixes".to_string(),
        "rejected".to_string(),
    ];
    assert_eq!(state_from_labels(&stacked, false), BoardState::Ready);
}

#[test]
fn stacked_accepted_rejected_decodes_accepted() {
    let stacked = vec!["accepted".to_string(), "rejected".to_string()];
    assert_eq!(state_from_labels(&stacked, false), BoardState::Accepted);
}

#[test]
fn stacked_needs_fixes_in_progress_decodes_in_progress() {
    // #1146 slice 3: stale "needs fixes" projects to InProgress.
    let stacked = vec!["in progress".to_string(), "needs fixes".to_string()];
    assert_eq!(state_from_labels(&stacked, false), BoardState::InProgress);
}

#[test]
fn stacked_ready_in_progress_decodes_in_progress() {
    let stacked = vec!["ready".to_string(), "in progress".to_string()];
    assert_eq!(state_from_labels(&stacked, false), BoardState::InProgress);
}

#[test]
fn stacked_delivered_ready_for_accept_decodes_to_delivered() {
    // #1146 slice 3: stale "ready for accept" projects to Delivered
    // (was display-equivalent to Delivered pre-collapse).
    let stacked = vec!["delivered".to_string(), "ready for accept".to_string()];
    assert_eq!(state_from_labels(&stacked, false), BoardState::Acceptance);
}

#[test]
fn ready_state_buckets_to_ready_column() {
    assert_eq!(column_of(&story_in(BoardState::Ready), None), Column::Ready);
}

#[test]
fn in_progress_state_buckets_to_in_progress_column() {
    assert_eq!(
        column_of(&story_in(BoardState::InProgress), None),
        Column::Middle(Stage::Dev.id())
    );
}

#[test]
fn retired_state_buckets_to_retired_column() {
    assert_eq!(
        column_of(&story_in(BoardState::Retired), None),
        Column::Retired,
        "concluded work lands in the RETIRED column so the ACCEPTED success-narrative stays clean"
    );
}

#[test]
fn empty_labels_open_decodes_icebox() {
    assert_eq!(state_from_labels(&[], false), BoardState::Icebox);
}

#[test]
fn empty_labels_closed_decodes_accepted() {
    assert_eq!(state_from_labels(&[], true), BoardState::Accepted);
}

#[test]
fn case_insensitive_label_matching() {
    // #1146 slice 3: stale "REJECTED" label projects to Ready
    // (post-collapse semantics; case-insensitivity preserved).
    let stacked = vec!["REJECTED".to_string(), "Ready For Accept".to_string()];
    assert_eq!(state_from_labels(&stacked, false), BoardState::Ready);
}
