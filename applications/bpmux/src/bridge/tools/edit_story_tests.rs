#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use crate::bridge::tools_stories::handle_edit_story;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::github::acceptor_from_labels;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{Story, StoryState};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn seed_story(number: u64, title: &str, body: &str) -> Story {
    let mut s = Story::new_minimal(
        number,
        title.to_string(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.body = Some(body.to_string());
    s
}

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch = SyncOrchestrator::new(store, adapter);
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch);
    orch.refresh_all().await.unwrap();
    orch
}

fn args_obj(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

#[tokio::test]
async fn edit_story_title_only_updates_title_preserves_body() {
    let refresh = build_refresh(vec![seed_story(
        1234,
        "[Feature] original title",
        "## What\n\nthe body content",
    )])
    .await;
    let args = args_obj(json!({
        "number": 1234,
        "title": "[Feature] corrected title"
    }));
    let result = handle_edit_story(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("edit_story with title-only succeeds");
    assert!(result.contains("title"), "result mentions title: {result}");
    let story = refresh.story(1234).expect("story still exists");
    assert_eq!(story.title, "[Feature] corrected title", "title updated");
    assert_eq!(
        story.body.as_deref(),
        Some("## What\n\nthe body content"),
        "body preserved"
    );
}

#[tokio::test]
async fn edit_story_body_only_updates_body_preserves_title() {
    let refresh = build_refresh(vec![seed_story(
        1234,
        "[Feature] original title",
        "old body",
    )])
    .await;
    let args = args_obj(json!({
        "number": 1234,
        "body": "## What\n\nnew body content"
    }));
    let result = handle_edit_story(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("edit_story with body-only succeeds");
    assert!(result.contains("body"), "result mentions body: {result}");
    let story = refresh.story(1234).expect("story still exists");
    assert_eq!(story.title, "[Feature] original title", "title preserved");
    assert_eq!(
        story.body.as_deref(),
        Some("## What\n\nnew body content"),
        "body updated"
    );
}

#[tokio::test]
async fn edit_story_with_both_title_and_body_updates_both() {
    let refresh = build_refresh(vec![seed_story(1234, "old title", "old body")]).await;
    let args = args_obj(json!({
        "number": 1234,
        "title": "new title",
        "body": "new body"
    }));
    let result = handle_edit_story(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("edit_story with both succeeds");
    assert!(
        result.contains("title") && result.contains("body"),
        "result mentions both: {result}"
    );
    let story = refresh.story(1234).expect("story still exists");
    assert_eq!(story.title, "new title", "title updated");
    assert_eq!(story.body.as_deref(), Some("new body"), "body updated");
}

#[tokio::test]
async fn edit_story_acceptor_resets_and_clears_the_stale_acceptor() {
    use crate::product_management::board::BoardEvent;
    use crate::product_management::board::events::EventMeta;
    use crate::product_management::story::Acceptor;

    let refresh = build_refresh(vec![seed_story(1234, "title", "body")]).await;
    let meta = EventMeta {
        actor: "pm".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    refresh
        .mutate_sync(BoardEvent::TagAdded {
            meta,
            number: 1234,
            label: "awaiting dev".to_string(),
        })
        .unwrap();
    refresh.drain_pending().await.unwrap();
    assert_eq!(
        acceptor_from_labels(&refresh.story(1234).unwrap().raw_labels),
        Some(Acceptor::Dev),
        "precondition: the create-derived acceptor is dev"
    );

    let args = args_obj(json!({ "number": 1234, "acceptor": "owner" }));
    handle_edit_story(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("edit_story with acceptor-only succeeds");

    let story = refresh.story(1234).unwrap();
    assert_eq!(
        acceptor_from_labels(&story.raw_labels),
        Some(Acceptor::Owner),
        "edit --acceptor owner re-derives the acceptor to owner"
    );
    assert!(
        !story.raw_labels.iter().any(|l| l == "awaiting dev"),
        "the stale 'awaiting dev' label is cleared (single-acceptor invariant); labels: {:?}",
        story.raw_labels
    );
}

#[tokio::test]
async fn edit_story_with_neither_field_rejects() {
    let refresh = build_refresh(vec![seed_story(1234, "title", "body")]).await;
    let args = args_obj(json!({ "number": 1234 }));
    let err = handle_edit_story(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect_err("edit_story with neither body, title, nor acceptor rejected");
    assert!(
        err.contains("at least one of 'body', 'title', or 'acceptor'"),
        "error names the required-one-of constraint: {err}"
    );
}

#[tokio::test]
async fn edit_story_rejects_non_pm_role() {
    let refresh = build_refresh(vec![seed_story(1234, "title", "body")]).await;
    let args = args_obj(json!({
        "number": 1234,
        "title": "new title"
    }));
    let err = handle_edit_story(&args, "dev:claude-code", Some(&refresh))
        .await
        .expect_err("dev role rejected from PM-only tool");
    assert!(
        err.to_lowercase().contains("pm") || err.to_lowercase().contains("role"),
        "error explains role-gating: {err}"
    );
}
