#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

// Tests for #1298: `iteration_id: Option<u64>` field + `list_iteration_children`
// MCP tool — slice 2 of Epic #1189 architectural-pivot. Sister-pattern to
// parent_epic + list_epic_children at the iteration-axis.

use super::list_iteration_children::handle_list_iteration_children;
use crate::board::BoardStore;
use crate::board::adapters::github::iteration_id_from_labels;
use crate::board::domain::{Story, StoryState, StoryType, story_to_board_json};
use chrono::Utc;
use std::sync::Arc;

fn iteration(number: u64) -> Story {
    Story {
        number,
        title: format!("Iteration #{number}"),
        state: StoryState::Icebox,
        story_type: StoryType::Iteration,
        tags: vec![],
        filed_at: Utc::now(),
        filed_by: "pm".to_string(),
        body: None,
        free_comments: None,
        gate_comments: None,
        is_closed: false,
        parent_epic: None,
        iteration_id: None,
        acceptance_test_path: None,
        raw_labels: vec!["iteration".to_string(), "icebox".to_string()],
        body_etag: None,
    }
}

fn member(number: u64, iter_id: u64, parent_epic: Option<u64>) -> Story {
    Story {
        number,
        title: format!("member #{number}"),
        state: StoryState::Ready,
        story_type: StoryType::Feature,
        tags: vec![],
        filed_at: Utc::now(),
        filed_by: "pm".to_string(),
        body: None,
        free_comments: None,
        gate_comments: None,
        is_closed: false,
        parent_epic,
        iteration_id: Some(iter_id),
        acceptance_test_path: None,
        raw_labels: vec![],
        body_etag: None,
    }
}

fn store_with(stories: Vec<Story>) -> Arc<BoardStore> {
    let store = BoardStore::new();
    store.seed_all(stories);
    Arc::new(store)
}

fn args(iteration_number: u64) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert(
        "iteration_number".to_string(),
        serde_json::json!(iteration_number),
    );
    m
}

#[tokio::test]
async fn list_iteration_children_returns_members_of_iteration() {
    let store = store_with(vec![
        iteration(1297),
        member(2001, 1297, None),
        member(2002, 1297, None),
        member(3001, 9999, None),
    ]);
    let out = handle_list_iteration_children(&args(1297), &store)
        .await
        .expect("list succeeds");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let children = parsed["children"].as_array().unwrap();
    assert_eq!(children.len(), 2, "only iteration-1297 members included");
    let numbers: Vec<u64> = children
        .iter()
        .map(|c| c["number"].as_u64().unwrap())
        .collect();
    assert!(numbers.contains(&2001));
    assert!(numbers.contains(&2002));
    assert!(!numbers.contains(&3001), "different iteration_id excluded");
}

#[tokio::test]
async fn list_iteration_children_preserves_canonical_number_order() {
    let store = store_with(vec![
        iteration(1297),
        member(2005, 1297, None),
        member(2001, 1297, None),
        member(2003, 1297, None),
    ]);
    let out = handle_list_iteration_children(&args(1297), &store)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let numbers: Vec<u64> = parsed["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["number"].as_u64().unwrap())
        .collect();
    assert_eq!(
        numbers,
        vec![2001, 2003, 2005],
        "sorted by number ascending"
    );
}

#[tokio::test]
async fn iteration_id_and_parent_epic_are_orthogonal_axes() {
    // Epic spans iterations naturally: same story can be in Epic AND in
    // Iteration without conflict (#1298 AC5: cross-axis preservation).
    let store = store_with(vec![
        iteration(1297),
        member(2001, 1297, Some(1189)),
        member(2002, 1297, Some(9999)),
        member(2003, 1297, None),
    ]);
    let out = handle_list_iteration_children(&args(1297), &store)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let children = parsed["children"].as_array().unwrap();
    assert_eq!(children.len(), 3, "all 3 are iteration-1297 members");
    let by_number: std::collections::HashMap<u64, &serde_json::Value> = children
        .iter()
        .map(|c| (c["number"].as_u64().unwrap(), c))
        .collect();
    assert_eq!(by_number[&2001]["parent_epic"].as_u64(), Some(1189));
    assert_eq!(by_number[&2002]["parent_epic"].as_u64(), Some(9999));
    assert!(
        by_number[&2003]["parent_epic"].is_null(),
        "no-parent-epic member still listed via iteration_id"
    );
}

#[tokio::test]
async fn iteration_label_decodes_to_iteration_id_field_roundtrip() {
    // Label-shortcut substrate per AC3 + falsifier-(d): iteration-#NNNN
    // label is the canonical source-of-truth that the adapter decodes
    // into the iteration_id field, AND story_to_board_json re-emits the
    // label from the field. Round-trip pins both directions.
    let labels = vec!["feature".to_string(), "iteration-#1297".to_string()];
    assert_eq!(iteration_id_from_labels(&labels), Some(1297));

    let mut s = member(2001, 1297, None);
    s.raw_labels = vec![];
    let board_json = story_to_board_json(&s);
    let emitted_labels: Vec<&str> = board_json["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        emitted_labels.contains(&"iteration-#1297"),
        "story_to_board_json emits iteration-#NNNN label from iteration_id field: {emitted_labels:?}"
    );
    assert_eq!(
        board_json["iteration_id"].as_u64(),
        Some(1297),
        "story_to_board_json emits iteration_id field"
    );

    assert!(
        iteration_id_from_labels(&["feature".to_string()]).is_none(),
        "no iteration-#NNNN label → None"
    );
    assert!(
        iteration_id_from_labels(&["iteration-#abc".to_string()]).is_none(),
        "malformed label → None"
    );
}

#[tokio::test]
async fn list_iteration_children_rejects_non_iteration_parent() {
    let store = store_with(vec![iteration(1297), {
        let mut s = iteration(1189);
        s.story_type = StoryType::Epic;
        s.raw_labels = vec!["epic".to_string()];
        s.title = "Epic #1189".to_string();
        s
    }]);
    let err = handle_list_iteration_children(&args(1189), &store)
        .await
        .expect_err("Epic rejected — not an Iteration");
    assert!(
        err.contains("not an Iteration"),
        "error names the type-mismatch: {err}"
    );
}

#[tokio::test]
async fn list_iteration_children_rejects_missing_iteration() {
    let store = store_with(vec![iteration(1297)]);
    let err = handle_list_iteration_children(&args(9999), &store)
        .await
        .expect_err("missing iteration_number rejected");
    assert!(
        err.contains("9999") && err.contains("not found"),
        "error names the missing number: {err}"
    );
}
