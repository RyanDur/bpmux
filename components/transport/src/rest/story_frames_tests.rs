#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::story_frames::diff_story_frames;
use message::StateDeltaPayload;

fn story(id: u64, state: &str) -> serde_json::Value {
    serde_json::json!({ "id": id, "title": format!("s{id}"), "state": state })
}

#[test]
fn an_unchanged_board_emits_nothing() {
    let board = vec![story(1, "ready"), story(2, "in-progress")];
    assert!(diff_story_frames(&board, &board).is_empty());
}

#[test]
fn one_changed_story_emits_one_upsert_never_the_array() {
    let prev = vec![story(1, "ready"), story(2, "in-progress")];
    let curr = vec![story(1, "ready"), story(2, "acceptance")];
    let frames = diff_story_frames(&prev, &curr);
    assert_eq!(frames.len(), 1);
    assert!(matches!(
        &frames[0],
        StateDeltaPayload::StoryUpserted { story } if story["id"] == 2 && story["state"] == "acceptance"
    ));
}

#[test]
fn a_new_story_emits_an_upsert_and_the_new_order() {
    let prev = vec![story(1, "ready")];
    let curr = vec![story(1, "ready"), story(3, "ready")];
    let frames = diff_story_frames(&prev, &curr);
    assert_eq!(frames.len(), 2);
    assert!(matches!(
        &frames[0],
        StateDeltaPayload::StoryUpserted { story } if story["id"] == 3
    ));
    assert!(matches!(
        &frames[1],
        StateDeltaPayload::StoryOrder { ids } if ids == &vec![1, 3]
    ));
}

#[test]
fn a_vanished_story_emits_a_removal_and_the_new_order() {
    let prev = vec![story(1, "ready"), story(2, "ready")];
    let curr = vec![story(2, "ready")];
    let frames = diff_story_frames(&prev, &curr);
    assert_eq!(frames.len(), 2);
    assert!(matches!(
        &frames[0],
        StateDeltaPayload::StoryRemoved { id: 1 }
    ));
    assert!(matches!(
        &frames[1],
        StateDeltaPayload::StoryOrder { ids } if ids == &vec![2]
    ));
}

#[test]
fn a_pure_reorder_emits_only_the_order_frame() {
    let prev = vec![story(1, "ready"), story(2, "ready"), story(3, "ready")];
    let curr = vec![story(3, "ready"), story(1, "ready"), story(2, "ready")];
    let frames = diff_story_frames(&prev, &curr);
    assert_eq!(
        frames.len(),
        1,
        "a reorder is one fact — never a re-shipment of untouched stories"
    );
    assert!(matches!(
        &frames[0],
        StateDeltaPayload::StoryOrder { ids } if ids == &vec![3, 1, 2]
    ));
}
