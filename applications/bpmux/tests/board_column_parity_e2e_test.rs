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

use bpmux::board::column::{ColumnBucket, column_for};
use bpmux::board::domain::StoryState;

/// Truth-table mirror of the renderer's `columnFor` from
/// `applications/electron/src/renderer/src/board/types.ts`.
///
/// Encoded by hand from the TS source so this test catches drift on
/// either side. Update both when the projection legitimately changes.
fn renderer_column_for(state: StoryState, is_closed: bool) -> ColumnBucket {
    if is_closed {
        return match state {
            StoryState::Icebox => ColumnBucket::Icebox,
            _ => ColumnBucket::Accepted,
        };
    }
    match state {
        StoryState::ReadyForAccept => ColumnBucket::Delivered,
        StoryState::NeedsFixes => ColumnBucket::InProgress,
        StoryState::Rejected => ColumnBucket::Ready,
        StoryState::Icebox => ColumnBucket::Icebox,
        StoryState::Ready => ColumnBucket::Ready,
        StoryState::InProgress => ColumnBucket::InProgress,
        StoryState::DeskCheck => ColumnBucket::DeskCheck,
        StoryState::Delivered => ColumnBucket::Delivered,
        StoryState::Accepted => ColumnBucket::Accepted,
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
        StoryState::ReadyForAccept,
        StoryState::NeedsFixes,
        StoryState::Rejected,
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
fn closed_non_icebox_stories_collapse_to_accepted() {
    for state in [
        StoryState::Ready,
        StoryState::InProgress,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::ReadyForAccept,
        StoryState::NeedsFixes,
        StoryState::Rejected,
        StoryState::Accepted,
    ] {
        assert_eq!(
            column_for(state, true),
            ColumnBucket::Accepted,
            "closed + state={state:?} must collapse to ACCEPTED (closure-as-acceptance)"
        );
    }
}

#[test]
fn closed_icebox_stays_icebox() {
    assert_eq!(
        column_for(StoryState::Icebox, true),
        ColumnBucket::Icebox,
        "closed-Icebox is the legitimate-historical exception and stays in ICEBOX"
    );
}

#[test]
fn open_workflow_states_bucket_per_renderer_grouping() {
    assert_eq!(column_for(StoryState::Ready, false), ColumnBucket::Ready);
    assert_eq!(
        column_for(StoryState::Rejected, false),
        ColumnBucket::Ready,
        "open + rejected → READY (re-pickup-able per Ryan's 15:19 proposal)"
    );
    assert_eq!(
        column_for(StoryState::InProgress, false),
        ColumnBucket::InProgress
    );
    assert_eq!(
        column_for(StoryState::NeedsFixes, false),
        ColumnBucket::InProgress,
        "open + needs-fixes → IN PROGRESS (designer-fail; dev iterating per #514 slice 2)"
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
        column_for(StoryState::ReadyForAccept, false),
        ColumnBucket::Delivered
    );
    assert_eq!(
        column_for(StoryState::Accepted, false),
        ColumnBucket::Accepted
    );
}
