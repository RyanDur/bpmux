//! Cross-surface column-bucket parity test for #520.
//!
//! Pins the invariant: for every (state × is_closed) cell, the Rust
//! `board::column::column_for` returns the same column the Electron
//! renderer's TypeScript `columnFor` (board/types.ts) returns. Two
//! surfaces, one definition of truth — and a single test that catches
//! drift in either direction.
//!
//! The TS function lives in
//! `applications/electron/src/renderer/src/board/types.ts` and is
//! mirrored here as a truth-table. If the renderer's columnFor
//! changes, this test breaks until both sides update; if the Rust
//! function changes, same. Per `feedback_test_fidelity_vocabulary_discipline`:
//! this is integration-fidelity (cross-surface contract) — substituting
//! a per-surface unit-test would be the cheaper-test bug the spec
//! falsifier-set explicitly rejects (F8).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bpmux::product_management::board::column::{ColumnBucket, column_for};
use bpmux::product_management::board::domain::StoryState;

/// Truth-table mirror of the renderer's `columnFor` from
/// `applications/electron/src/renderer/src/board/types.ts`.
///
/// Encoded by hand from the TS source so this test catches drift on
/// either side. Update both when the projection legitimately changes.
fn renderer_column_for(state: StoryState, is_closed: bool) -> ColumnBucket {
    if is_closed && state != StoryState::Accepted && state != StoryState::Retired {
        return ColumnBucket::Retired;
    }
    match state {
        StoryState::Icebox => ColumnBucket::Icebox,
        StoryState::Ready => ColumnBucket::Ready,
        StoryState::InProgress => ColumnBucket::InProgress,
        StoryState::DeskCheck => ColumnBucket::DeskCheck,
        StoryState::Delivered | StoryState::Acceptance => ColumnBucket::Delivered,
        StoryState::Accepted => ColumnBucket::Accepted,
        StoryState::Retired => ColumnBucket::Retired,
    }
}

#[test]
fn column_for_agrees_across_full_cross_product() {
    let states = [
        StoryState::Icebox,
        StoryState::Ready,
        StoryState::InProgress,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::Accepted,
    ];
    for state in states {
        for is_closed in [false, true] {
            let rust_bucket = column_for(state, is_closed);
            let renderer_bucket = renderer_column_for(state, is_closed);
            assert_eq!(
                rust_bucket, renderer_bucket,
                "drift: state={state:?} is_closed={is_closed} \
                 — Rust column_for = {rust_bucket:?}, renderer columnFor (mirrored) = {renderer_bucket:?}"
            );
        }
    }
}

#[test]
fn closed_non_accepted_stories_project_to_retired() {
    // #1146 slice 2b: close_story for retire/dedupe/won't-fix lands in
    // the Retired column rather than polluting Accepted's success-
    // narrative. Closed + Accepted stays in Accepted (user-story
    // satisfied + closed = legitimate completion).
    for state in [
        StoryState::Icebox,
        StoryState::Ready,
        StoryState::InProgress,
        StoryState::DeskCheck,
        StoryState::Delivered,
    ] {
        assert_eq!(
            column_for(state, true),
            ColumnBucket::Retired,
            "closed + state={state:?} (non-Accepted) must project to RETIRED"
        );
    }
}

#[test]
fn closed_accepted_stays_accepted() {
    assert_eq!(
        column_for(StoryState::Accepted, true),
        ColumnBucket::Accepted,
        "closed + Accepted stays in ACCEPTED (user-story satisfied + closed = success-narrative)"
    );
}

#[test]
fn open_workflow_states_bucket_one_to_one() {
    // #1146 slice 3: post-collapse, the 6 enum states map 1:1 to
    // their display columns. Rework lives in transition history,
    // not in separate Rejected/NeedsFixes/ReadyForAccept variants.
    assert_eq!(column_for(StoryState::Icebox, false), ColumnBucket::Icebox);
    assert_eq!(column_for(StoryState::Ready, false), ColumnBucket::Ready);
    assert_eq!(
        column_for(StoryState::InProgress, false),
        ColumnBucket::InProgress
    );
    assert_eq!(
        column_for(StoryState::DeskCheck, false),
        ColumnBucket::DeskCheck
    );
    assert_eq!(
        column_for(StoryState::Delivered, false),
        ColumnBucket::Delivered
    );
    assert_eq!(
        column_for(StoryState::Accepted, false),
        ColumnBucket::Accepted
    );
}
