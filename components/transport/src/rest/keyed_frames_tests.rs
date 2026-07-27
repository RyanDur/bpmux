#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::keyed_frames::diff_keyed;

fn item(id: u64, name: &str) -> serde_json::Value {
    serde_json::json!({ "id": id, "name": name })
}

#[test]
fn an_unchanged_collection_diffs_to_nothing() {
    let items = vec![item(1, "a"), item(2, "b")];
    let diff = diff_keyed(&items, &items);
    assert!(diff.upserts.is_empty());
    assert!(diff.removed.is_empty());
    assert!(diff.order.is_none());
}

#[test]
fn one_changed_item_is_one_upsert() {
    let prev = vec![item(1, "a"), item(2, "b")];
    let curr = vec![item(1, "a"), item(2, "b2")];
    let diff = diff_keyed(&prev, &curr);
    assert_eq!(diff.upserts, vec![item(2, "b2")]);
    assert!(diff.removed.is_empty());
    assert!(diff.order.is_none());
}

#[test]
fn additions_and_removals_come_with_the_new_order() {
    let prev = vec![item(1, "a"), item(2, "b")];
    let curr = vec![item(3, "c"), item(1, "a")];
    let diff = diff_keyed(&prev, &curr);
    assert_eq!(diff.upserts, vec![item(3, "c")]);
    assert_eq!(diff.removed, vec![2]);
    assert_eq!(diff.order, Some(vec![3, 1]));
}

#[test]
fn a_pure_reorder_is_only_an_order() {
    let prev = vec![item(1, "a"), item(2, "b")];
    let curr = vec![item(2, "b"), item(1, "a")];
    let diff = diff_keyed(&prev, &curr);
    assert!(diff.upserts.is_empty());
    assert!(diff.removed.is_empty());
    assert_eq!(diff.order, Some(vec![2, 1]));
}
