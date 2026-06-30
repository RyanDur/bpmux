#![allow(clippy::unwrap_used, clippy::expect_used)]

use bpmux::product_management::board::adapters::github::state_from_labels;
use bpmux::product_management::board::column::{ColumnBucket, column_for};
use bpmux::product_management::story::Stage;
use bpmux::product_management::story::StoryState;

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
    assert_eq!(state_from_labels(&stacked, false), StoryState::Ready);
}

#[test]
fn stacked_accepted_rejected_decodes_accepted() {
    let stacked = vec!["accepted".to_string(), "rejected".to_string()];
    assert_eq!(state_from_labels(&stacked, false), StoryState::Accepted);
}

#[test]
fn stacked_needs_fixes_in_progress_decodes_in_progress() {
    // #1146 slice 3: stale "needs fixes" projects to InProgress.
    let stacked = vec!["in progress".to_string(), "needs fixes".to_string()];
    assert_eq!(
        state_from_labels(&stacked, false),
        StoryState::InProgress(Stage::Dev)
    );
}

#[test]
fn stacked_ready_in_progress_decodes_in_progress() {
    let stacked = vec!["ready".to_string(), "in progress".to_string()];
    assert_eq!(
        state_from_labels(&stacked, false),
        StoryState::InProgress(Stage::Dev)
    );
}

#[test]
fn stacked_delivered_ready_for_accept_decodes_to_delivered() {
    // #1146 slice 3: stale "ready for accept" projects to Delivered
    // (was display-equivalent to Delivered pre-collapse).
    let stacked = vec!["delivered".to_string(), "ready for accept".to_string()];
    assert_eq!(state_from_labels(&stacked, false), StoryState::Delivered);
}

#[test]
fn ready_state_buckets_to_ready_column() {
    // #1146 slice 3: post-collapse, stale rejected labels project to
    // Ready at parse-time, so the bucketing test now operates on the
    // surviving Ready state.
    let bucket = column_for(StoryState::Ready, false);
    assert_eq!(bucket, ColumnBucket::Ready);
}

#[test]
fn in_progress_state_buckets_to_in_progress_column() {
    let bucket = column_for(StoryState::InProgress(Stage::Dev), false);
    assert_eq!(bucket, ColumnBucket::InProgress);
}

#[test]
fn closed_ready_buckets_to_retired_column() {
    // #1146 slice 3: closed stories from any non-Accepted state project
    // to Retired. The pre-collapse Rejected variant becomes Ready at
    // parse-time and then Retired at column-projection when closed.
    let bucket = column_for(StoryState::Ready, true);
    assert_eq!(
        bucket,
        ColumnBucket::Retired,
        "Closed-non-Accepted projects to RETIRED so success-narrative stays in ACCEPTED"
    );
}

#[test]
fn empty_labels_open_decodes_icebox() {
    assert_eq!(state_from_labels(&[], false), StoryState::Icebox);
}

#[test]
fn empty_labels_closed_decodes_accepted() {
    assert_eq!(state_from_labels(&[], true), StoryState::Accepted);
}

#[test]
fn case_insensitive_label_matching() {
    // #1146 slice 3: stale "REJECTED" label projects to Ready
    // (post-collapse semantics; case-insensitivity preserved).
    let stacked = vec!["REJECTED".to_string(), "Ready For Accept".to_string()];
    assert_eq!(state_from_labels(&stacked, false), StoryState::Ready);
}
