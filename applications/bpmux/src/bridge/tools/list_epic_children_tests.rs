#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::list_epic_children::handle_list_epic_children;
use crate::product_management::board::BoardStore;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::adapters::github::decode_story_metadata;
use crate::product_management::story::{Story, StoryKind, StoryState, StoryType};
use serde_json::json;
use std::sync::Arc;

fn decoded(number: u64, title: &str, labels: &[&str]) -> Story {
    let label_json: Vec<serde_json::Value> = labels.iter().map(|n| json!({ "name": n })).collect();
    decode_story_metadata(&json!({
        "number": number,
        "title": title,
        "gh_state": "open",
        "labels": label_json,
        "created_at": "2026-05-15T01:00:00+00:00",
        "user": { "login": "pm" },
    }))
    .expect("issue JSON must decode")
}

fn epic(number: u64) -> Story {
    decoded(number, &format!("Epic #{number}"), &["epic", "icebox"])
}

fn child(number: u64, parent: u64, story_type: StoryType, state: StoryState) -> Story {
    let parent_label = format!("parent-epic-#{parent}");
    let type_label = GhLabelCodec.encode_story_type(story_type);
    let mut labels = vec![parent_label.as_str(), type_label.as_str()];
    if state != StoryState::Icebox {
        labels.push(state.github_label());
    }
    decoded(number, &format!("child #{number}"), &labels)
}

fn child_without_parent(number: u64) -> Story {
    decoded(number, &format!("orphan #{number}"), &["feature"])
}

fn store_with(stories: Vec<Story>) -> Arc<BoardStore> {
    let store = BoardStore::new();
    store.seed_all(stories);
    Arc::new(store)
}

fn args(epic_id: u64) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("epic_id".to_string(), serde_json::json!(epic_id));
    m
}

#[tokio::test]
async fn returns_zero_children_for_empty_epic() {
    let store = store_with(vec![epic(100)]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["epic_id"], 100);
    assert_eq!(parsed["children"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn returns_single_child_with_parent_epic_match() {
    let store = store_with(vec![
        epic(100),
        child(
            101,
            100,
            StoryType::Story(StoryKind::Feature),
            StoryState::Ready,
        ),
    ]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let children = parsed["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], 101);
    assert_eq!(children[0]["state"], "ready");
    assert_eq!(children[0]["story_type"], "feature");
    assert_eq!(children[0]["is_closed"], false);
}

#[tokio::test]
async fn returns_multiple_children_sorted_by_number() {
    let store = store_with(vec![
        epic(100),
        child(
            105,
            100,
            StoryType::Story(StoryKind::Feature),
            StoryState::Ready,
        ),
        child(
            102,
            100,
            StoryType::Story(StoryKind::Bug),
            StoryState::Icebox,
        ),
        child(
            108,
            100,
            StoryType::Story(StoryKind::Chore),
            StoryState::Accepted,
        ),
    ]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let children = parsed["children"].as_array().unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0]["id"], 102);
    assert_eq!(children[1]["id"], 105);
    assert_eq!(children[2]["id"], 108);
}

#[tokio::test]
async fn excludes_stories_with_different_parent() {
    let store = store_with(vec![
        epic(100),
        epic(200),
        child(
            101,
            100,
            StoryType::Story(StoryKind::Feature),
            StoryState::Ready,
        ),
        child(
            201,
            200,
            StoryType::Story(StoryKind::Feature),
            StoryState::Ready,
        ),
    ]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let children = parsed["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], 101);
}

#[tokio::test]
async fn excludes_stories_without_parent_epic() {
    let store = store_with(vec![
        epic(100),
        child(
            101,
            100,
            StoryType::Story(StoryKind::Feature),
            StoryState::Ready,
        ),
        child_without_parent(202),
    ]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["children"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn rejects_non_epic_story_type() {
    let not_epic = decoded(100, "feature #100", &["feature", "icebox"]);
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
    assert!(err.contains("epic_id"), "got: {err}");
}

#[tokio::test]
async fn handles_large_epic_with_ten_children() {
    let mut stories = vec![epic(100)];
    for n in 1..=10 {
        stories.push(child(
            100 + n,
            100,
            StoryType::Story(StoryKind::Feature),
            StoryState::Icebox,
        ));
    }
    let store = store_with(stories);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["children"].as_array().unwrap().len(), 10);
}

#[tokio::test]
async fn surfaces_is_closed_state() {
    let mut accepted = child(
        101,
        100,
        StoryType::Story(StoryKind::Feature),
        StoryState::Accepted,
    );
    accepted.is_closed = true;
    let store = store_with(vec![epic(100), accepted]);
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["children"][0]["is_closed"], true);
    assert_eq!(parsed["children"][0]["state"], "accepted");
}
