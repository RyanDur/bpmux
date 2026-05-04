#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! #1165 ATP — Background poller-middleware hydrates canonical-store;
//! read tools always serve from canonical (no live-GH-fetch on read paths).
//!
//! Slice 1: get_story canonical-only — removes lazy-fetch fall-through
//! that hit GH live when story.body or gate_comments were missing.
//! Slices 2-4 (poller-middleware substrate / ETag + since / stale state +
//! #1141 supersede) layer on top in subsequent commits.

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState};
use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::bridge::tools::handle_get_story;

fn seed_minimal(number: u64, state: StoryState) -> Story {
    Story::new_minimal(
        number,
        format!("[Feature] target-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[tokio::test]
async fn get_story_canonical_only_returns_placeholder_when_body_missing() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    store.seed_all(vec![seed_minimal(8001, StoryState::Ready)]);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(8001));
    let result = handle_get_story(&args, Some(&store), Some(&refresh)).await;
    assert!(
        result.is_ok(),
        "get_story succeeds without live GH-fetch even when body is None: {result:?}"
    );
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["number"], 8001);
    assert_eq!(json["body_hydrated"], false);
    assert!(
        json["body"]
            .as_str()
            .unwrap()
            .contains("body not yet hydrated"),
        "missing-body returns placeholder copy: {}",
        json["body"]
    );
}

#[tokio::test]
async fn get_story_serves_cached_body_from_canonical_store() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut story = seed_minimal(8002, StoryState::Ready);
    story.body = Some("# real body\n\ncached body content from prior hydrate".to_string());
    store.seed_all(vec![story]);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(8002));
    let result = handle_get_story(&args, Some(&store), Some(&refresh)).await;
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["body_hydrated"], true);
    assert!(
        json["body"]
            .as_str()
            .unwrap()
            .contains("cached body content"),
        "cached body served from canonical: {}",
        json["body"]
    );
}

#[tokio::test]
async fn get_story_does_not_call_adapter_fetch_body_when_body_missing() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    store.seed_all(vec![seed_minimal(8003, StoryState::InProgress)]);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(8003));
    let result = handle_get_story(&args, Some(&store), Some(&refresh)).await;
    assert!(
        result.is_ok(),
        "get_story does not invoke adapter.fetch_body even though body is missing + adapter would fail: {result:?}"
    );
}

#[tokio::test]
async fn get_story_returns_gate_comments_hydrated_flag_in_response() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    store.seed_all(vec![seed_minimal(8004, StoryState::Ready)]);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(8004));
    let result = handle_get_story(&args, Some(&store), Some(&refresh)).await;
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(
        json["gate_comments_hydrated"], false,
        "fresh seed has no gate_comments hydrated; field exposed for caller to discover"
    );
}
