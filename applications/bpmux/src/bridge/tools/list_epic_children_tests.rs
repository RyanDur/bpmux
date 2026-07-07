#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::list_epic_children::handle_list_epic_children;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::integrations::github::mapper::{
    decode_epic_from_story, decode_story_metadata, is_epic_story,
};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::epic::Epic;
use crate::product_management::story::{BoardState, Story, StoryKind};
use crate::sync::{RefreshDriver, SyncOrchestrator};
use serde_json::json;
use std::sync::Arc;

fn decoded(number: u64, title: &str, labels: &[&str]) -> (Story, Vec<String>) {
    let label_json: Vec<serde_json::Value> = labels.iter().map(|n| json!({ "name": n })).collect();
    let story = decode_story_metadata(&json!({
        "number": number,
        "title": title,
        "gh_state": "open",
        "labels": label_json,
        "created_at": "2026-05-15T01:00:00+00:00",
        "user": { "login": "pm" },
    }))
    .expect("issue JSON must decode");
    (story, labels.iter().map(ToString::to_string).collect())
}

fn epic(number: u64) -> (Story, Vec<String>) {
    decoded(number, &format!("Epic #{number}"), &["epic", "icebox"])
}

fn child(
    number: u64,
    parent: u64,
    story_type: StoryKind,
    state: BoardState,
) -> (Story, Vec<String>) {
    let parent_label = format!("parent-epic-#{parent}");
    let type_label = GhLabelCodec.encode_story_type(story_type);
    let mut labels = vec![parent_label.as_str(), type_label.as_str()];
    if state != BoardState::Icebox {
        labels.push(state.github_label());
    }
    decoded(number, &format!("child #{number}"), &labels)
}

fn child_without_parent(number: u64) -> (Story, Vec<String>) {
    decoded(number, &format!("orphan #{number}"), &["feature"])
}

async fn store_with(seeds: Vec<(Story, Vec<String>)>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let epics: Vec<Epic> = seeds
        .iter()
        .filter(|(story, labels)| is_epic_story(story, labels))
        .map(|(story, _)| decode_epic_from_story(story))
        .collect();
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], epics);
    orch.refresh_all().await.unwrap();
    orch
}

fn args(epic_id: u64) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("epic_id".to_string(), serde_json::json!(epic_id));
    m
}

#[tokio::test]
async fn returns_zero_children_for_empty_epic() {
    let store = store_with(vec![epic(100)]).await;
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["epic_id"], 100);
    assert_eq!(parsed["children"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn returns_single_child_with_parent_epic_match() {
    let store = store_with(vec![
        epic(100),
        child(101, 100, StoryKind::Feature, BoardState::Ready),
    ])
    .await;
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
        child(105, 100, StoryKind::Feature, BoardState::Ready),
        child(102, 100, StoryKind::Bug, BoardState::Icebox),
        child(108, 100, StoryKind::Chore, BoardState::Accepted),
    ])
    .await;
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
        child(101, 100, StoryKind::Feature, BoardState::Ready),
        child(201, 200, StoryKind::Feature, BoardState::Ready),
    ])
    .await;
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
        child(101, 100, StoryKind::Feature, BoardState::Ready),
        child_without_parent(202),
    ])
    .await;
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["children"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn rejects_non_epic_story_type() {
    let not_epic = decoded(100, "feature #100", &["feature", "icebox"]);
    let store = store_with(vec![not_epic]).await;
    let err = handle_list_epic_children(&args(100), &store)
        .await
        .unwrap_err();
    assert!(err.contains("not an Epic"), "got: {err}");
}

#[tokio::test]
async fn rejects_non_existent_epic_number() {
    let store = store_with(vec![]).await;
    let err = handle_list_epic_children(&args(999), &store)
        .await
        .unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}

#[tokio::test]
async fn rejects_missing_epic_number_arg() {
    let store = store_with(vec![]).await;
    let empty = serde_json::Map::new();
    let err = handle_list_epic_children(&empty, &store).await.unwrap_err();
    assert!(err.contains("epic_id"), "got: {err}");
}

#[tokio::test]
async fn handles_large_epic_with_ten_children() {
    let mut stories = vec![epic(100)];
    for n in 1..=10 {
        stories.push(child(100 + n, 100, StoryKind::Feature, BoardState::Icebox));
    }
    let store = store_with(stories).await;
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["children"].as_array().unwrap().len(), 10);
}

#[tokio::test]
async fn derives_is_closed_from_workflow_conclusion() {
    let (accepted, accepted_labels) = child(101, 100, StoryKind::Feature, BoardState::Accepted);
    let store = store_with(vec![epic(100), (accepted, accepted_labels)]).await;
    let result = handle_list_epic_children(&args(100), &store).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed["children"][0]["is_closed"], true,
        "is_closed derives from the workflow conclusion (Accepted|Retired), not from a stored GH mirror"
    );
    assert_eq!(parsed["children"][0]["state"], "accepted");
}
