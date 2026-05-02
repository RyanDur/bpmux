//! Regression test for #509: stacked workflow-state labels must decode
//! to the highest-priority canonical state per state_from_labels. This
//! covers the patterns from the 6 today's-rejects named in the spec
//! (#469 / #463 / #464 / #450 / #457 / #498) — each carrying ["ready
//! for accept", "needs fixes", "rejected"] on GH at filing time.
//!
//! Today's substrate-arc (#519 universal-reap on reject_story, #503
//! universal-reap on pass/fail_desk_check, #520+#522 column-projection
//! parity) prevents NEW stacked-label states from forming. This test
//! locks in the decoder's correctness for any pre-existing stories
//! that still carry stacked labels — they decode correctly to the
//! highest-priority state, and column_for buckets them where Ryan's
//! 15:19 UTC proposal asked (rejected → READY-with-feedback).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bpmux::board::adapters::github::state_from_labels;
use bpmux::board::column::{ColumnBucket, column_for};
use bpmux::board::domain::StoryState;

#[test]
fn stacked_rejected_needs_fixes_ready_for_accept_decodes_rejected() {
    let stacked = vec![
        "ready for accept".to_string(),
        "needs fixes".to_string(),
        "rejected".to_string(),
    ];
    assert_eq!(state_from_labels(&stacked, false), StoryState::Rejected);
}

#[test]
fn stacked_accepted_rejected_decodes_accepted() {
    let stacked = vec!["accepted".to_string(), "rejected".to_string()];
    assert_eq!(state_from_labels(&stacked, false), StoryState::Accepted);
}

#[test]
fn stacked_needs_fixes_in_progress_decodes_needs_fixes() {
    let stacked = vec!["in progress".to_string(), "needs fixes".to_string()];
    assert_eq!(state_from_labels(&stacked, false), StoryState::NeedsFixes);
}

#[test]
fn stacked_ready_in_progress_decodes_in_progress() {
    let stacked = vec!["ready".to_string(), "in progress".to_string()];
    assert_eq!(state_from_labels(&stacked, false), StoryState::InProgress);
}

#[test]
fn stacked_delivered_ready_for_accept_decodes_ready_for_accept() {
    let stacked = vec!["delivered".to_string(), "ready for accept".to_string()];
    assert_eq!(
        state_from_labels(&stacked, false),
        StoryState::ReadyForAccept
    );
}

#[test]
fn rejected_state_buckets_to_ready_column_per_ryan_proposal() {
    let bucket = column_for(StoryState::Rejected, false);
    assert_eq!(
        bucket,
        ColumnBucket::Ready,
        "Per Ryan 2026-04-27 15:19 UTC: rejected stories surface in READY-with-feedback for re-pickup"
    );
}

#[test]
fn needs_fixes_state_buckets_to_in_progress_column() {
    let bucket = column_for(StoryState::NeedsFixes, false);
    assert_eq!(bucket, ColumnBucket::InProgress);
}

#[test]
fn closed_rejected_buckets_to_retired_column() {
    let bucket = column_for(StoryState::Rejected, true);
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
    let stacked = vec!["REJECTED".to_string(), "Ready For Accept".to_string()];
    assert_eq!(state_from_labels(&stacked, false), StoryState::Rejected);
}
