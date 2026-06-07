#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
async fn select_stories_to_hydrate_picks_active_states_with_missing_body() {
    use crate::bridge::select_stories_to_hydrate;
    let mut active = seed_minimal(100, StoryState::Ready);
    active.body = None;
    let mut hydrated = seed_minimal(101, StoryState::InProgress);
    hydrated.body = Some("already-cached".to_string());
    let mut accepted = seed_minimal(102, StoryState::Accepted);
    accepted.body = None;
    let mut icebox = seed_minimal(103, StoryState::Icebox);
    icebox.body = None;
    let mut delivered = seed_minimal(104, StoryState::Delivered);
    delivered.body = None;
    let stories = vec![active, hydrated, accepted, icebox, delivered];
    let picks = select_stories_to_hydrate(&stories, 10);
    assert!(picks.contains(&100), "active+missing picked: {picks:?}");
    assert!(picks.contains(&104), "delivered+missing picked: {picks:?}");
    assert!(!picks.contains(&101), "hydrated stories skipped: {picks:?}");
    assert!(
        !picks.contains(&102),
        "accepted stories not prioritized for hydration: {picks:?}"
    );
    assert!(
        !picks.contains(&103),
        "icebox stories not prioritized for hydration: {picks:?}"
    );
}

#[tokio::test]
async fn select_stories_to_hydrate_caps_at_limit() {
    use crate::bridge::select_stories_to_hydrate;
    let stories: Vec<Story> = (0..10)
        .map(|i| {
            let mut s = seed_minimal(200 + i, StoryState::Ready);
            s.body = None;
            s
        })
        .collect();
    let picks = select_stories_to_hydrate(&stories, 3);
    assert_eq!(picks.len(), 3, "limit caps per-tick fetch volume");
}

#[tokio::test]
async fn select_stories_to_hydrate_returns_empty_when_all_hydrated() {
    use crate::bridge::select_stories_to_hydrate;
    let mut s = seed_minimal(300, StoryState::Ready);
    s.body = Some("already".to_string());
    let picks = select_stories_to_hydrate(&[s], 5);
    assert!(picks.is_empty(), "no missing-body stories = empty picks");
}

#[tokio::test]
async fn body_fetched_event_with_etag_stores_etag_on_canonical_story() {
    use crate::board::events::BoardEvent;
    let store = BoardStore::new();
    store.seed_all(vec![seed_minimal(9000, StoryState::Ready)]);
    store.apply(&BoardEvent::BodyFetched {
        meta: crate::board::orchestrator::cache_fill_meta(),
        number: 9000,
        body: "hydrated body content".to_string(),
        etag: Some("W/\"deadbeef\"".to_string()),
    });
    let story = store.story(9000).expect("story present");
    assert_eq!(story.body.as_deref(), Some("hydrated body content"));
    assert_eq!(story.body_etag.as_deref(), Some("W/\"deadbeef\""));
}

#[tokio::test]
async fn local_body_mutate_invalidates_prior_etag() {
    use crate::board::events::{BoardEvent, EventMeta};
    use crate::board::peer::PeerId;
    let store = BoardStore::new();
    let mut seed = seed_minimal(9100, StoryState::Ready);
    seed.body = Some("v1".to_string());
    seed.body_etag = Some("W/\"etag-v1\"".to_string());
    store.seed_all(vec![seed.clone()]);
    let mut updated = seed;
    updated.body = Some("v2".to_string());
    store.apply(&BoardEvent::StoryRefreshed {
        meta: EventMeta {
            actor: "test".to_string(),
            ts: Utc::now(),
            peer_id: PeerId::local(),
        },
        number: 9100,
        story: Box::new(updated),
    });
    let after = store.story(9100).expect("story present");
    assert_eq!(after.body.as_deref(), Some("v2"));
    assert_eq!(
        after.body_etag, None,
        "local mutate cleared prior etag so next conditional poll re-fetches"
    );
}

#[tokio::test]
async fn fetch_body_conditional_dispatches_event_with_etag_on_modified() {
    use crate::board::events::BoardEvent;
    use crate::board::sync::ConditionalBody;
    use crate::board::{RefreshDriver, SyncOrchestrator};
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_minimal(9001, StoryState::Ready)]);
    adapter.seed_body(9001, "real body".into());
    let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orch.ingest_once().await.unwrap();
    let mut rx = store.subscribe();
    let result = RefreshDriver::fetch_body_conditional(&orch, 9001, None)
        .await
        .expect("conditional fetch ok");
    assert!(matches!(result, ConditionalBody::Modified { .. }));
    let event = rx.recv().await.expect("event dispatched");
    assert!(matches!(
        event,
        BoardEvent::BodyFetched { number: 9001, .. }
    ));
    assert_eq!(
        store.story(9001).unwrap().body.as_deref(),
        Some("real body")
    );
}

#[tokio::test]
async fn select_stories_to_revalidate_picks_already_hydrated_active_stories() {
    use crate::bridge::select_stories_to_revalidate;
    let mut active_with_body = seed_minimal(700, StoryState::Ready);
    active_with_body.body = Some("body".to_string());
    active_with_body.body_etag = Some("etag-700".to_string());
    let mut active_no_body = seed_minimal(701, StoryState::InProgress);
    active_no_body.body = None;
    let mut accepted_with_body = seed_minimal(702, StoryState::Accepted);
    accepted_with_body.body = Some("body".to_string());
    let mut icebox_with_body = seed_minimal(703, StoryState::Icebox);
    icebox_with_body.body = Some("body".to_string());
    let stories = vec![
        active_with_body,
        active_no_body,
        accepted_with_body,
        icebox_with_body,
    ];
    let picks = select_stories_to_revalidate(&stories, 10);
    assert!(
        picks
            .iter()
            .any(|(n, e)| *n == 700 && e.as_deref() == Some("etag-700")),
        "active+hydrated stories selected with their etag: {picks:?}"
    );
    assert!(
        !picks.iter().any(|(n, _)| *n == 701),
        "missing-body stories left to hydration loop: {picks:?}"
    );
    assert!(
        !picks.iter().any(|(n, _)| *n == 702),
        "accepted stories not revalidated: {picks:?}"
    );
    assert!(
        !picks.iter().any(|(n, _)| *n == 703),
        "icebox stories not revalidated: {picks:?}"
    );
}

#[tokio::test]
async fn select_stories_to_revalidate_caps_at_limit() {
    use crate::bridge::select_stories_to_revalidate;
    let stories: Vec<Story> = (0..10)
        .map(|i| {
            let mut s = seed_minimal(800 + i, StoryState::Ready);
            s.body = Some("body".to_string());
            s
        })
        .collect();
    let picks = select_stories_to_revalidate(&stories, 2);
    assert_eq!(picks.len(), 2, "limit caps revalidation volume");
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
        json["gate_comments_hydrated"], true,
        "#1686: get_story read-through-fetches gate-records for a rework-state story; the flag reports fetched (empty here — 8004 has no gates)"
    );
    assert!(
        json.get("latest_reject")
            .map(|x| x.is_null())
            .unwrap_or(true),
        "a non-rejected story surfaces no latest_reject even though its gate-records are now hydrated"
    );
}
