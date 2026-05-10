#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

//! #1258 close_iteration α-substrate extension: tests for the
//! cross-store atomic step (promote stories above topmost marker,
//! delete topmost marker, drop unfinished ready-stories to icebox-top).
//!
//! Function under test: `crate::session::priority::close_iteration_atomic`.

use crate::session::iteration_markers::IterationMarker;
use crate::session::priority::{
    CloseIterationOutcome, IceboxItem, PriorityIndex, close_iteration_atomic,
};

fn marker(id: u64, position: u32) -> IterationMarker {
    IterationMarker {
        id,
        theme_name: format!("theme{id}"),
        position,
        created_at: "2026-05-10T00:00:00Z".to_string(),
    }
}

#[test]
fn no_markers_in_icebox_is_noop_outcome() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    let mut markers: Vec<IterationMarker> = vec![];
    let snapshot_unified = index.icebox_unified.clone();
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[], false);
    assert_eq!(outcome, CloseIterationOutcome::default());
    assert_eq!(outcome.deleted_marker_id, None);
    assert!(outcome.promoted.is_empty());
    assert!(outcome.dropped_back.is_empty());
    assert_eq!(index.icebox_unified, snapshot_unified);
}

#[test]
fn promotes_stories_above_first_marker_to_ready() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 300 });
    let mut markers = vec![marker(7, 0), marker(8, 1)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[], false);
    assert_eq!(outcome.promoted, vec![100, 200]);
    assert_eq!(
        index.columns.get("ready").cloned().unwrap_or_default(),
        vec![100, 200]
    );
}

#[test]
fn deletes_only_first_marker_keeps_remaining_markers() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 8 });
    let mut markers = vec![marker(7, 0), marker(8, 1)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[], false);
    assert_eq!(outcome.deleted_marker_id, Some(7));
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].id, 8);
    // Remaining icebox_unified preserves story 200 + marker 8.
    assert!(
        index
            .icebox_unified
            .contains(&IceboxItem::Story { number: 200 })
    );
    assert!(index.icebox_unified.contains(&IceboxItem::Marker { id: 8 }));
    assert!(!index.icebox_unified.contains(&IceboxItem::Marker { id: 7 }));
}

#[test]
fn drops_back_unfinished_ready_to_top_of_icebox() {
    let mut index = PriorityIndex::default();
    index
        .columns
        .insert("ready".to_string(), vec![900, 901, 100]);
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    let mut markers = vec![marker(7, 0)];
    // 900 + 901 are unfinished ready-stories; 100 is a story still in ready
    // (e.g., promoted earlier this iteration but not started).
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[900, 901], false);
    assert_eq!(outcome.dropped_back, vec![900, 901]);
    // Dropped-back stories prepend to icebox_unified.
    assert_eq!(index.icebox_unified[0], IceboxItem::Story { number: 900 });
    assert_eq!(index.icebox_unified[1], IceboxItem::Story { number: 901 });
    // Dropped-back stories removed from ready column.
    let ready = index.columns.get("ready").cloned().unwrap_or_default();
    assert!(!ready.contains(&900));
    assert!(!ready.contains(&901));
}

#[test]
fn marker_at_top_of_icebox_promotes_zero_stories_but_deletes_marker() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    let mut markers = vec![marker(7, 0)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[], false);
    assert!(outcome.promoted.is_empty());
    assert_eq!(outcome.deleted_marker_id, Some(7));
    assert_eq!(markers.len(), 0);
    // Story 200 stays in icebox_unified post-marker-delete.
    assert!(
        index
            .icebox_unified
            .contains(&IceboxItem::Story { number: 200 })
    );
}

#[test]
fn full_flow_combines_drop_back_promote_delete() {
    let mut index = PriorityIndex::default();
    // Substrate-consistent setup: 900 is the unfinished ready-story
    // from prior iteration. 1100/1200/1300 live in icebox_unified
    // awaiting iteration assignment.
    index.columns.insert("ready".to_string(), vec![900]);
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1100 });
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1300 });
    index.icebox_unified.push(IceboxItem::Marker { id: 8 });
    let mut markers = vec![marker(7, 0), marker(8, 1)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[900], false);
    // Promote: 1100 + 1200 (above marker 7) → ready.
    assert_eq!(outcome.promoted, vec![1100, 1200]);
    // Drop-back: 900 → top of icebox.
    assert_eq!(outcome.dropped_back, vec![900]);
    // Delete: marker 7.
    assert_eq!(outcome.deleted_marker_id, Some(7));
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].id, 8);
    // icebox_unified now: [Story 900, Story 1300, Marker 8].
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 900 },
            IceboxItem::Story { number: 1300 },
            IceboxItem::Marker { id: 8 },
        ]
    );
    // Ready column: 900 dropped back, 1100/1200 promoted in.
    let ready = index.columns.get("ready").cloned().unwrap_or_default();
    assert_eq!(ready, vec![1100, 1200]);
}

#[test]
fn empty_icebox_with_only_markers_promotes_zero_deletes_first_marker() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Marker { id: 8 });
    let mut markers = vec![marker(7, 0), marker(8, 1)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[], false);
    assert!(outcome.promoted.is_empty());
    assert_eq!(outcome.deleted_marker_id, Some(7));
    assert_eq!(markers.len(), 1);
    assert_eq!(index.icebox_unified, vec![IceboxItem::Marker { id: 8 }]);
}

#[test]
fn no_unfinished_ready_means_empty_dropped_back() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[], false);
    assert!(outcome.dropped_back.is_empty());
}

// ── AC 19: carry_over=true keeps unfinished in ready (5 ready: 2 unfinished + 3 promoted) ──

#[test]
fn carry_over_true_keeps_unfinished_in_ready_with_promoted() {
    let mut index = PriorityIndex::default();
    // 2 unfinished + 3 about to be promoted
    index.columns.insert("ready".to_string(), vec![900, 901]);
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1100 });
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1200 });
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1300 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[900, 901], true);
    assert_eq!(outcome.promoted, vec![1100, 1200, 1300]);
    // carry_over=true: dropped_back is empty.
    assert!(outcome.dropped_back.is_empty());
    let ready = index.columns.get("ready").cloned().unwrap_or_default();
    // Ready now contains all 5: 2 unfinished + 3 promoted.
    assert_eq!(ready, vec![900, 901, 1100, 1200, 1300]);
    // icebox_unified empty (only Marker was there above + 3 stories, all consumed).
    assert!(index.icebox_unified.is_empty());
}

#[test]
fn carry_over_false_default_drops_back_unfinished() {
    let mut index = PriorityIndex::default();
    index.columns.insert("ready".to_string(), vec![900, 901]);
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1100 });
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1200 });
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1300 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[900, 901], false);
    assert_eq!(outcome.promoted, vec![1100, 1200, 1300]);
    assert_eq!(outcome.dropped_back, vec![900, 901]);
    let ready = index.columns.get("ready").cloned().unwrap_or_default();
    // 900/901 dropped back; 1100/1200/1300 promoted.
    assert_eq!(ready, vec![1100, 1200, 1300]);
    // icebox_unified now: [Story 900, Story 901].
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 900 },
            IceboxItem::Story { number: 901 },
        ]
    );
}

// ── AC 29: Bug/Chore bypass preserved (regression — function ignores non-ready columns) ──

#[test]
fn bug_chore_class_in_other_columns_unaffected() {
    // Stories outside "ready" + "icebox_unified" are not touched; the
    // function operates only on those two surfaces. This validates
    // Bug/Chore-class bypass-to-ready isn't disturbed by close_iteration.
    let mut index = PriorityIndex::default();
    index
        .columns
        .insert("in-progress".to_string(), vec![500, 501]);
    index.columns.insert("desk-check".to_string(), vec![600]);
    index
        .columns
        .insert("delivered".to_string(), vec![700, 701]);
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[], false);
    assert_eq!(outcome.promoted, vec![100]);
    // Other columns untouched.
    assert_eq!(
        index.columns.get("in-progress").cloned(),
        Some(vec![500, 501])
    );
    assert_eq!(index.columns.get("desk-check").cloned(), Some(vec![600]));
    assert_eq!(
        index.columns.get("delivered").cloned(),
        Some(vec![700, 701])
    );
}

// ── AC 32: scope-of-unfinished — in_progress / desk-check / delivered NOT touched ──

#[test]
fn scope_unfinished_excludes_in_progress_desk_check_delivered() {
    // Caller passes only ready-column members as unfinished_ready.
    // close_iteration_atomic operates only on what's passed.
    let mut index = PriorityIndex::default();
    index.columns.insert("ready".to_string(), vec![100, 101]);
    index.columns.insert("in-progress".to_string(), vec![200]);
    index.columns.insert("desk-check".to_string(), vec![300]);
    index.columns.insert("delivered".to_string(), vec![400]);
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    // Pass only ready-column members — caller scope discipline.
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[100, 101], false);
    assert_eq!(outcome.dropped_back, vec![100, 101]);
    // Only ready was touched; in_progress/desk-check/delivered unchanged.
    let ready = index.columns.get("ready").cloned().unwrap_or_default();
    assert!(ready.is_empty());
    assert_eq!(index.columns.get("in-progress").cloned(), Some(vec![200]));
    assert_eq!(index.columns.get("desk-check").cloned(), Some(vec![300]));
    assert_eq!(index.columns.get("delivered").cloned(), Some(vec![400]));
    // Icebox-top now has 100, 101.
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 101 },
        ]
    );
}
