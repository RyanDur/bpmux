#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use crate::bridge::tools::apply_remove_stories_from_iteration;
use crate::session::priority::{IceboxItem, load_priority};

fn seed_priority(path: &std::path::Path, json: &str) {
    std::fs::write(path, json).unwrap();
}

#[test]
fn single_story_removal_relocates_below_bottom_marker() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"marker","id":7},
            {"kind":"story","number":900}
        ]}"#,
    );

    let outcome = apply_remove_stories_from_iteration(&priority_path, dir.path(), &[100]).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 900 },
            IceboxItem::Story { number: 100 },
        ]
    );
    assert_eq!(outcome.stories.len(), 1);
    assert_eq!(outcome.stories[0].id, 100);
    assert_eq!(outcome.stories[0].position, 3);
}

#[test]
fn batch_removal_relocates_all_below_bottom_marker_preserving_input_order() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"story","number":200},
            {"kind":"story","number":300},
            {"kind":"marker","id":7}
        ]}"#,
    );

    let outcome =
        apply_remove_stories_from_iteration(&priority_path, dir.path(), &[100, 200, 300]).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
            IceboxItem::Story { number: 300 },
        ]
    );
    assert_eq!(outcome.stories.len(), 3);
}

#[test]
fn preserves_marker_positions() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"marker","id":7},
            {"kind":"story","number":200},
            {"kind":"marker","id":8}
        ]}"#,
    );

    apply_remove_stories_from_iteration(&priority_path, dir.path(), &[100, 200]).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Marker { id: 7 },
            IceboxItem::Marker { id: 8 },
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
        ]
    );
}

#[test]
fn preserves_other_stories_in_their_iteration_groups() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"story","number":200},
            {"kind":"marker","id":7},
            {"kind":"story","number":300}
        ]}"#,
    );

    apply_remove_stories_from_iteration(&priority_path, dir.path(), &[100]).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 200 },
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 300 },
            IceboxItem::Story { number: 100 },
        ]
    );
}

#[test]
fn handles_story_not_in_iteration_as_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":200},
            {"kind":"marker","id":7}
        ]}"#,
    );

    let outcome = apply_remove_stories_from_iteration(&priority_path, dir.path(), &[999]).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 200 },
            IceboxItem::Marker { id: 7 },
        ]
    );
    assert!(outcome.stories.is_empty());
}

#[test]
fn idempotent_on_already_unscheduled_story() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"marker","id":7},
            {"kind":"story","number":300}
        ]}"#,
    );

    let outcome = apply_remove_stories_from_iteration(&priority_path, dir.path(), &[300]).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 300 },
        ]
    );
    assert_eq!(outcome.stories.len(), 1);
    assert_eq!(outcome.stories[0].position, 2);
}

#[test]
fn idempotent_second_call_is_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"marker","id":7}
        ]}"#,
    );

    apply_remove_stories_from_iteration(&priority_path, dir.path(), &[100]).unwrap();
    let snapshot = load_priority(&priority_path);
    apply_remove_stories_from_iteration(&priority_path, dir.path(), &[100]).unwrap();
    let after = load_priority(&priority_path);
    assert_eq!(snapshot.icebox_unified, after.icebox_unified);
}

#[test]
fn no_markers_treats_story_as_already_unscheduled() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"story","number":200}
        ]}"#,
    );

    apply_remove_stories_from_iteration(&priority_path, dir.path(), &[100]).unwrap();

    let index = load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
        ]
    );
}

#[test]
fn empty_story_ids_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    seed_priority(&priority_path, r#"{"icebox_unified":[]}"#);

    let result = apply_remove_stories_from_iteration(&priority_path, dir.path(), &[]);
    assert!(result.is_err());
}

#[test]
fn save_failure_rolls_back_to_snapshot() {
    let priority_path = std::path::PathBuf::from("/dev/null/no_such_priority_dir/priority.json");
    let session_dir = std::path::PathBuf::from("/dev/null/no_such_session_dir");

    let result = apply_remove_stories_from_iteration(&priority_path, &session_dir, &[100]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("save priority") || err.contains("rolled back"),
        "save-failure must surface in error: {err}"
    );
}
