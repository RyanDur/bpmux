#![allow(clippy::unwrap_used)]

use super::list_epic_children::handle_list_epic_children;
use crate::board::BoardStore;
use crate::board::domain::{Story, StoryState, StoryType};
use chrono::Utc;
use std::sync::Arc;

fn epic(number: u64) -> Story {
    Story {
        number,
        title: format!("Epic #{number}"),
        state: StoryState::Icebox,
        story_type: StoryType::Epic,
        tags: vec![],
        filed_at: Utc::now(),
        filed_by: "pm".to_string(),
        body: None,
        free_comments: None,
        gate_comments: None,
        is_closed: false,
        parent_epic: None,
        acceptance_test_path: None,
        raw_labels: vec!["Epic".to_string(), "icebox".to_string()],
        body_etag: None,
        iteration_membership: None,
        iteration_order: None,
        iteration_review_closed_by: Vec::new(),
        iteration_dev_ok_by: Vec::new(),
        horizon: None,
        is_current: false,
        acceptor: None,
    }
}

fn child(number: u64, parent: u64, story_type: StoryType, state: StoryState) -> Story {
    Story {
        number,
        title: format!("child #{number}"),
        state,
        story_type,
        tags: vec![],
        filed_at: Utc::now(),
        filed_by: "pm".to_string(),
        body: None,
        free_comments: None,
        gate_comments: None,
        is_closed: false,
        parent_epic: Some(parent),
        acceptance_test_path: None,
        raw_labels: vec![],
        body_etag: None,
        iteration_membership: None,
        iteration_order: None,
        iteration_review_closed_by: Vec::new(),
        iteration_dev_ok_by: Vec::new(),
        horizon: None,
        is_current: false,
        acceptor: None,
    }
}

fn store_with(stories: Vec<Story>) -> Arc<BoardStore> {
    let store = BoardStore::new();
    store.seed_all(stories);
    Arc::new(store)
}

fn args(epic_number: u64) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("epic_number".to_string(), serde_json::json!(epic_number));
    m
}

#[tokio::test]
async fn returns_zero_children_for_empty_epic() {
    let store = store_with(vec![epic(100)]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["epic_number"], 100);
    assert_eq!(parsed["children"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn returns_single_child_with_parent_epic_match() {
    let store = store_with(vec![
        epic(100),
        child(101, 100, StoryType::Feature, StoryState::Ready),
    ]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let children = parsed["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["number"], 101);
    assert_eq!(children[0]["state"], "ready");
    assert_eq!(children[0]["story_type"], "feature");
    assert_eq!(children[0]["is_closed"], false);
}

#[tokio::test]
async fn returns_multiple_children_sorted_by_number() {
    let store = store_with(vec![
        epic(100),
        child(105, 100, StoryType::Feature, StoryState::Ready),
        child(102, 100, StoryType::Bug, StoryState::Icebox),
        child(108, 100, StoryType::Chore, StoryState::Accepted),
    ]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let children = parsed["children"].as_array().unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0]["number"], 102);
    assert_eq!(children[1]["number"], 105);
    assert_eq!(children[2]["number"], 108);
}

#[tokio::test]
async fn excludes_stories_with_different_parent() {
    let store = store_with(vec![
        epic(100),
        epic(200),
        child(101, 100, StoryType::Feature, StoryState::Ready),
        child(201, 200, StoryType::Feature, StoryState::Ready),
    ]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let children = parsed["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["number"], 101);
}

#[tokio::test]
async fn excludes_stories_without_parent_epic() {
    let store = store_with(vec![
        epic(100),
        child(101, 100, StoryType::Feature, StoryState::Ready),
        Story {
            parent_epic: None,
            ..child(202, 999, StoryType::Feature, StoryState::Ready)
        },
    ]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["children"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn rejects_non_epic_story_type() {
    let mut not_epic = epic(100);
    not_epic.story_type = StoryType::Feature;
    let store = store_with(vec![not_epic]);
    let err = handle_list_epic_children(&args(100), &store)
        .await
        .unwrap_err();
    assert!(err.contains("not an Epic"), "got: {err}");
}

#[tokio::test]
async fn rejects_non_existent_epic_number() {
    let store = store_with(vec![]);
    let err = handle_list_epic_children(&args(999), &store)
        .await
        .unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}

#[tokio::test]
async fn rejects_missing_epic_number_arg() {
    let store = store_with(vec![]);
    let empty = serde_json::Map::new();
    let err = handle_list_epic_children(&empty, &store).await.unwrap_err();
    assert!(err.contains("epic_number"), "got: {err}");
}

#[tokio::test]
async fn handles_large_epic_with_ten_children() {
    let mut stories = vec![epic(100)];
    for n in 1..=10 {
        stories.push(child(100 + n, 100, StoryType::Feature, StoryState::Icebox));
    }
    let store = store_with(stories);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["children"].as_array().unwrap().len(), 10);
}

#[tokio::test]
async fn surfaces_is_closed_state() {
    let mut accepted = child(101, 100, StoryType::Feature, StoryState::Accepted);
    accepted.is_closed = true;
    let store = store_with(vec![epic(100), accepted]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["children"][0]["is_closed"], true);
    assert_eq!(parsed["children"][0]["state"], "accepted");
}
