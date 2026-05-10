#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use crate::bridge::tools::apply_add_stories_to_iteration;
use crate::session::iteration_markers::{IterationMarker, IterationMarkersStore, save};
use crate::session::priority::{IceboxItem, load_priority};

fn seed_priority(path: &std::path::Path, json: &str) {
    std::fs::write(path, json).unwrap();
}

fn seed_markers(session_dir: &std::path::Path, markers: Vec<IterationMarker>, next_id: u64) {
    let store = IterationMarkersStore { markers, next_id };
    save(session_dir, &store).unwrap();
}

fn empty_priority_json() -> &'static str {
    r#"{"icebox_unified":[]}"#
}

#[test]
fn fold_with_marker_id_positions_stories_above_marker_in_input_order() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[{"kind":"marker","id":7}]}"#,
    );
    seed_markers(
        session_dir.path(),
        vec![IterationMarker {
            id: 7,
            theme_name: "Orion".to_string(),
            position: 0,
            created_at: "ts".to_string(),
        }],
        8,
    );

    let outcome = apply_add_stories_to_iteration(
        &priority_path,
        session_dir.path(),
        &[100, 200, 300],
        Some(7),
        None,
    )
    .unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
            IceboxItem::Story { number: 300 },
            IceboxItem::Marker { id: 7 },
        ]
    );
    assert_eq!(outcome.stories.len(), 3);
    assert_eq!(outcome.stories[0].id, 100);
    assert_eq!(outcome.stories[0].position, 1);
    assert_eq!(outcome.marker.id, 7);
    assert_eq!(outcome.marker.position, 4);
    assert_eq!(outcome.marker.theme_name, "Orion");
}

#[test]
fn fold_with_theme_name_creates_marker_then_positions_stories_above_it() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(&priority_path, empty_priority_json());

    let outcome = apply_add_stories_to_iteration(
        &priority_path,
        session_dir.path(),
        &[100, 200],
        None,
        Some("Lyra".to_string()),
    )
    .unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
            IceboxItem::Marker {
                id: outcome.marker.id
            },
        ]
    );
    assert_eq!(outcome.marker.theme_name, "Lyra");
    assert_eq!(outcome.marker.position, 3);
    assert_eq!(outcome.stories[0].position, 1);
    assert_eq!(outcome.stories[1].position, 2);
}

#[test]
fn fold_preserves_existing_items_above_target_marker() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":900},
            {"kind":"marker","id":7}
        ]}"#,
    );
    seed_markers(
        session_dir.path(),
        vec![IterationMarker {
            id: 7,
            theme_name: "Orion".to_string(),
            position: 0,
            created_at: "ts".to_string(),
        }],
        8,
    );

    apply_add_stories_to_iteration(&priority_path, session_dir.path(), &[100], Some(7), None)
        .unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 900 },
            IceboxItem::Story { number: 100 },
            IceboxItem::Marker { id: 7 },
        ]
    );
}

#[test]
fn fold_evicts_legacy_columns_icebox_entry_for_each_story() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{
            "icebox": [100, 200, 300, 400],
            "icebox_unified": [{"kind":"marker","id":7}]
        }"#,
    );
    seed_markers(
        session_dir.path(),
        vec![IterationMarker {
            id: 7,
            theme_name: "Orion".to_string(),
            position: 0,
            created_at: "ts".to_string(),
        }],
        8,
    );

    apply_add_stories_to_iteration(
        &priority_path,
        session_dir.path(),
        &[100, 200],
        Some(7),
        None,
    )
    .unwrap();

    let index = load_priority(&priority_path);
    let legacy = index.columns.get("icebox").cloned().unwrap_or_default();
    assert!(!legacy.contains(&100));
    assert!(!legacy.contains(&200));
    assert!(legacy.contains(&300));
    assert!(legacy.contains(&400));
}

#[test]
fn fold_with_marker_id_wins_when_both_marker_id_and_theme_name_provided() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[{"kind":"marker","id":7}]}"#,
    );
    seed_markers(
        session_dir.path(),
        vec![IterationMarker {
            id: 7,
            theme_name: "Orion".to_string(),
            position: 0,
            created_at: "ts".to_string(),
        }],
        8,
    );

    let outcome = apply_add_stories_to_iteration(
        &priority_path,
        session_dir.path(),
        &[100],
        Some(7),
        Some("Lyra".to_string()),
    )
    .unwrap();

    assert_eq!(outcome.marker.id, 7);
    assert_eq!(outcome.marker.theme_name, "Orion");
    let store = crate::session::iteration_markers::load(session_dir.path());
    assert_eq!(
        store.markers.len(),
        1,
        "no new marker created when marker_id provided"
    );
}

#[test]
fn fold_returns_error_when_neither_marker_id_nor_theme_name_provided() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(&priority_path, empty_priority_json());

    let result =
        apply_add_stories_to_iteration(&priority_path, session_dir.path(), &[100], None, None);
    assert!(result.is_err());
}

#[test]
fn fold_returns_error_when_story_ids_empty() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[{"kind":"marker","id":7}]}"#,
    );
    seed_markers(
        session_dir.path(),
        vec![IterationMarker {
            id: 7,
            theme_name: "Orion".to_string(),
            position: 0,
            created_at: "ts".to_string(),
        }],
        8,
    );

    let result =
        apply_add_stories_to_iteration(&priority_path, session_dir.path(), &[], Some(7), None);
    assert!(result.is_err());
}

#[test]
fn fold_returns_error_when_marker_id_not_in_icebox_unified() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(&priority_path, empty_priority_json());

    let result =
        apply_add_stories_to_iteration(&priority_path, session_dir.path(), &[100], Some(99), None);
    assert!(result.is_err());
}

#[test]
fn fold_existing_story_in_unified_moves_to_target_position() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"story","number":900},
            {"kind":"marker","id":7}
        ]}"#,
    );
    seed_markers(
        session_dir.path(),
        vec![IterationMarker {
            id: 7,
            theme_name: "Orion".to_string(),
            position: 0,
            created_at: "ts".to_string(),
        }],
        8,
    );

    apply_add_stories_to_iteration(&priority_path, session_dir.path(), &[100], Some(7), None)
        .unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 900 },
            IceboxItem::Story { number: 100 },
            IceboxItem::Marker { id: 7 },
        ]
    );
}

#[test]
fn fold_save_priority_persisted_atomic_when_markers_save_succeeds() {
    let priority_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(&priority_path, empty_priority_json());

    let outcome = apply_add_stories_to_iteration(
        &priority_path,
        session_dir.path(),
        &[100, 200],
        None,
        Some("Lyra".to_string()),
    )
    .unwrap();

    let priority_index = load_priority(&priority_path);
    let markers_store = crate::session::iteration_markers::load(session_dir.path());
    assert_eq!(priority_index.icebox_unified.len(), 3);
    assert_eq!(markers_store.markers.len(), 1);
    assert_eq!(markers_store.markers[0].id, outcome.marker.id);
}

#[test]
fn markers_save_failure_rolls_back_priority_to_snapshot() {
    let priority_dir = tempfile::tempdir().unwrap();
    let priority_path = priority_dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[{"kind":"marker","id":7}]}"#,
    );
    let snapshot_before_call = load_priority(&priority_path);
    let unwritable_session_dir =
        std::path::PathBuf::from("/dev/null/no_such_session_dir_for_rollback_test");

    let result = apply_add_stories_to_iteration(
        &priority_path,
        &unwritable_session_dir,
        &[100, 200],
        Some(7),
        None,
    );
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("priority rolled back") || err.contains("rollback"),
        "rollback path must surface in error message: {err}"
    );
    let priority_index = load_priority(&priority_path);
    assert_eq!(
        priority_index.icebox_unified, snapshot_before_call.icebox_unified,
        "priority must match pre-call snapshot after markers-save-failure rollback"
    );
}

#[test]
fn priority_save_failure_short_circuits_before_markers_save() {
    let priority_path = std::path::PathBuf::from("/dev/null/no_such_priority_dir/priority.json");
    let session_dir = tempfile::tempdir().unwrap();

    let result = apply_add_stories_to_iteration(
        &priority_path,
        session_dir.path(),
        &[100, 200],
        None,
        Some("Lyra".to_string()),
    );
    assert!(result.is_err());
    let markers_store = crate::session::iteration_markers::load(session_dir.path());
    assert!(
        markers_store.markers.is_empty(),
        "markers store must be untouched when priority save fails first"
    );
}
