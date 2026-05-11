#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use crate::bridge::tools::apply_reorder_iteration_marker;
use crate::session::priority::{IceboxItem, load_priority};

#[test]
fn marker_reorder_lands_in_icebox_unified_at_target_position() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    std::fs::write(&path, r#"{"icebox_unified":[{"kind":"marker","id":7}]}"#).unwrap();
    apply_reorder_iteration_marker(&path, dir.path(), 7, 1).unwrap();
    let index = load_priority(&path);
    assert_eq!(index.icebox_unified, vec![IceboxItem::Marker { id: 7 }]);
}

#[test]
fn marker_reorder_moves_marker_across_stories_to_position_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    std::fs::write(
        &path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"story","number":200},
            {"kind":"marker","id":7}
        ]}"#,
    )
    .unwrap();
    apply_reorder_iteration_marker(&path, dir.path(), 7, 1).unwrap();
    let index = load_priority(&path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
        ]
    );
}

#[test]
fn marker_reorder_moves_marker_below_existing_stories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    std::fs::write(
        &path,
        r#"{"icebox_unified":[
            {"kind":"marker","id":7},
            {"kind":"story","number":100},
            {"kind":"story","number":200}
        ]}"#,
    )
    .unwrap();
    apply_reorder_iteration_marker(&path, dir.path(), 7, 3).unwrap();
    let index = load_priority(&path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
            IceboxItem::Marker { id: 7 },
        ]
    );
}

#[test]
fn marker_reorder_position_clamped_to_unified_length() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    std::fs::write(
        &path,
        r#"{"icebox_unified":[{"kind":"story","number":100},{"kind":"marker","id":7}]}"#,
    )
    .unwrap();
    apply_reorder_iteration_marker(&path, dir.path(), 7, 99).unwrap();
    let index = load_priority(&path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Marker { id: 7 },
        ]
    );
}

#[test]
fn marker_reorder_preserves_other_markers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    std::fs::write(
        &path,
        r#"{"icebox_unified":[
            {"kind":"marker","id":7},
            {"kind":"story","number":100},
            {"kind":"marker","id":8}
        ]}"#,
    )
    .unwrap();
    apply_reorder_iteration_marker(&path, dir.path(), 8, 1).unwrap();
    let index = load_priority(&path);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Marker { id: 8 },
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 100 },
        ]
    );
}
