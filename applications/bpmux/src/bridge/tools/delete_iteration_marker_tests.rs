#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use crate::bridge::tools::apply_delete_iteration_marker;
use crate::session::iteration_markers::{IterationMarker, IterationMarkersStore, save};
use crate::session::priority::{IceboxItem, load_priority};

fn seed_priority(path: &std::path::Path, json: &str) {
    std::fs::write(path, json).unwrap();
}

fn seed_markers(session_dir: &std::path::Path, markers: Vec<IterationMarker>, next_id: u64) {
    let store = IterationMarkersStore { markers, next_id };
    save(session_dir, &store).unwrap();
}

fn marker(id: u64, theme: &str, position: u32) -> IterationMarker {
    IterationMarker {
        id,
        theme_name: theme.to_string(),
        position,
        created_at: "2026-05-10T22:00:00Z".to_string(),
        ..IterationMarker::default()
    }
}

#[test]
fn delete_empty_marker_succeeds_and_removes_from_both_stores() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[{"kind":"marker","id":7}]}"#,
    );
    seed_markers(session_dir.path(), vec![marker(7, "Orion", 0)], 8);

    let outcome = apply_delete_iteration_marker(&priority_path, session_dir.path(), 7).unwrap();

    assert_eq!(outcome.deleted_marker_id, 7);
    assert_eq!(outcome.theme_name, "Orion");
    let index = load_priority(&priority_path);
    assert!(index.icebox_unified.is_empty());
    let store = crate::session::iteration_markers::load(session_dir.path());
    assert!(store.markers.is_empty());
}

#[test]
fn delete_marker_with_stories_above_errors_with_named_blockers_and_preserves_state() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"story","number":200},
            {"kind":"marker","id":7}
        ]}"#,
    );
    seed_markers(session_dir.path(), vec![marker(7, "Orion", 0)], 8);
    let snapshot = load_priority(&priority_path);

    let result = apply_delete_iteration_marker(&priority_path, session_dir.path(), 7);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("#100"), "error names blocker #100: {err}");
    assert!(err.contains("#200"), "error names blocker #200: {err}");
    assert!(err.contains("Orion"), "error names theme: {err}");
    let after = load_priority(&priority_path);
    assert_eq!(after.icebox_unified, snapshot.icebox_unified);
    let store = crate::session::iteration_markers::load(session_dir.path());
    assert_eq!(store.markers.len(), 1);
}

#[test]
fn delete_non_existent_marker_errors() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(&priority_path, r#"{"icebox_unified":[]}"#);
    seed_markers(session_dir.path(), vec![], 1);

    let result = apply_delete_iteration_marker(&priority_path, session_dir.path(), 99);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("99"), "error names missing marker_id: {err}");
}

#[test]
fn delete_preserves_other_markers() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"marker","id":7},
            {"kind":"marker","id":8},
            {"kind":"marker","id":9}
        ]}"#,
    );
    seed_markers(
        session_dir.path(),
        vec![
            marker(7, "Orion", 0),
            marker(8, "Lyra", 1),
            marker(9, "Aquila", 2),
        ],
        10,
    );

    apply_delete_iteration_marker(&priority_path, session_dir.path(), 8).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![IceboxItem::Marker { id: 7 }, IceboxItem::Marker { id: 9 }]
    );
    let store = crate::session::iteration_markers::load(session_dir.path());
    let ids: Vec<u64> = store.markers.iter().map(|m| m.id).collect();
    assert_eq!(ids, vec![7, 9]);
}

#[test]
fn delete_marker_with_stories_only_in_other_groups_succeeds() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"marker","id":7},
            {"kind":"marker","id":8}
        ]}"#,
    );
    seed_markers(
        session_dir.path(),
        vec![marker(7, "Orion", 0), marker(8, "Lyra", 1)],
        9,
    );

    apply_delete_iteration_marker(&priority_path, session_dir.path(), 8).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Marker { id: 7 },
        ]
    );
}

#[test]
fn delete_marker_with_stories_in_its_group_but_not_topmost_errors() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"marker","id":7},
            {"kind":"story","number":200},
            {"kind":"marker","id":8}
        ]}"#,
    );
    seed_markers(
        session_dir.path(),
        vec![marker(7, "Orion", 0), marker(8, "Lyra", 1)],
        9,
    );

    let result = apply_delete_iteration_marker(&priority_path, session_dir.path(), 8);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("#200"), "error names blocker #200: {err}");
}

#[test]
fn save_failure_returns_error_with_rollback() {
    let priority_path = std::path::PathBuf::from("/dev/null/no_such_priority_dir/priority.json");
    let session_dir = tempfile::tempdir().unwrap();

    let result = apply_delete_iteration_marker(&priority_path, session_dir.path(), 7);

    assert!(result.is_err());
}
