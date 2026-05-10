#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

//! #1256 server-side α-substrate cohesion for iteration-markers.
//!
//! Post-α (#1258): markers live in PriorityIndex.icebox_unified, the
//! unified positional space for the icebox column. This file holds
//! tests asserting the substrate-truth: when a marker is created via
//! the canonical mutation chain, it appears in icebox_unified so all
//! readers (renderer via /api/board, future API clients) see it.

use crate::session::iteration_markers::{
    IterationMarker, IterationMarkersStore, add as add_marker, save as save_markers,
};
use crate::session::priority::{IceboxItem, PriorityIndex, load_priority, save_priority};

/// Simulates the post_iteration_marker mutation chain at the substrate
/// layer (skipping HTTP framing). Mirrors the canonical chain in
/// transport::rest::post_iteration_marker so tests can assert the
/// substrate-truth invariant without spinning up axum.
fn create_marker_with_icebox_unified_append(
    session_dir: &std::path::Path,
    priority_path: &std::path::Path,
    theme: &str,
) -> IterationMarker {
    let mut store = IterationMarkersStore::default();
    if let Ok(content) = std::fs::read_to_string(session_dir.join("iteration_markers.json"))
        && let Ok(parsed) = serde_json::from_str::<IterationMarkersStore>(&content)
    {
        store = parsed;
    }
    let now = "2026-05-10T04:00:00Z".to_string();
    let marker = add_marker(&mut store, theme.to_string(), now);
    save_markers(session_dir, &store).unwrap();

    let mut index = load_priority(priority_path);
    index
        .icebox_unified
        .push(IceboxItem::Marker { id: marker.id });
    save_priority(priority_path, &index).unwrap();

    marker
}

#[test]
fn post_iteration_marker_appends_to_icebox_unified() {
    let session_dir = tempfile::tempdir().unwrap();
    let priority_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");

    let marker =
        create_marker_with_icebox_unified_append(session_dir.path(), &priority_path, "Orion");

    let index = load_priority(&priority_path);
    assert!(
        index
            .icebox_unified
            .contains(&IceboxItem::Marker { id: marker.id }),
        "newly-created marker must appear in icebox_unified canonical ordering"
    );
}

#[test]
fn marker_created_after_existing_unified_state_appends_at_bottom() {
    // Pre-existing icebox_unified contains a story + a marker. New
    // marker should append at the bottom (preserving priority order).
    let session_dir = tempfile::tempdir().unwrap();
    let priority_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    save_priority(&priority_path, &index).unwrap();

    let new_marker =
        create_marker_with_icebox_unified_append(session_dir.path(), &priority_path, "Lyra");

    let post = load_priority(&priority_path);
    assert_eq!(post.icebox_unified.len(), 3);
    assert_eq!(post.icebox_unified[0], IceboxItem::Story { number: 100 });
    assert_eq!(post.icebox_unified[1], IceboxItem::Marker { id: 7 });
    assert_eq!(
        post.icebox_unified[2],
        IceboxItem::Marker { id: new_marker.id }
    );
}

#[test]
fn multiple_markers_each_appended_distinct_in_icebox_unified() {
    let session_dir = tempfile::tempdir().unwrap();
    let priority_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");

    let m1 = create_marker_with_icebox_unified_append(session_dir.path(), &priority_path, "Orion");
    let m2 = create_marker_with_icebox_unified_append(session_dir.path(), &priority_path, "Lyra");
    let m3 =
        create_marker_with_icebox_unified_append(session_dir.path(), &priority_path, "Cassiopeia");

    let index = load_priority(&priority_path);
    let marker_ids: Vec<u64> = index
        .icebox_unified
        .iter()
        .filter_map(|item| match item {
            IceboxItem::Marker { id } => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(marker_ids, vec![m1.id, m2.id, m3.id]);
}

#[test]
fn marker_create_preserves_existing_columns() {
    // post_iteration_marker mutation must NOT touch other state-columns.
    let session_dir = tempfile::tempdir().unwrap();
    let priority_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    let mut index = PriorityIndex::default();
    index.columns.insert("ready".to_string(), vec![100, 200]);
    index.columns.insert("in-progress".to_string(), vec![300]);
    save_priority(&priority_path, &index).unwrap();

    create_marker_with_icebox_unified_append(session_dir.path(), &priority_path, "Orion");

    let post = load_priority(&priority_path);
    assert_eq!(post.columns.get("ready").cloned(), Some(vec![100, 200]));
    assert_eq!(post.columns.get("in-progress").cloned(), Some(vec![300]));
}
