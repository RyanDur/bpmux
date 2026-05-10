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
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[]);
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
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[]);
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
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[]);
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
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[900, 901]);
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
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[]);
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
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[900]);
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
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[]);
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
    let outcome = close_iteration_atomic(&mut index, &mut markers, &[]);
    assert!(outcome.dropped_back.is_empty());
}
