#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use crate::bridge::tools::handle_get_story;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{BoardState, Stage, Story, StoryId};
use crate::store::{AppAction, SeededStore};
use crate::sync::{RefreshDriver, SyncOrchestrator};

fn seed_minimal(id: u64, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("[Feature] target-{id}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[tokio::test]
async fn get_story_canonical_only_returns_placeholder_when_body_missing() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let seed = SeededStore::new(vec![], vec![], vec![]);
    seed.bind_refresh(&refresh);
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_minimal(
        8001,
        BoardState::Ready,
    )])));
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(8001));
    let result = handle_get_story(&args, Some(&refresh)).await;
    assert!(
        result.is_ok(),
        "get_story succeeds without live GH-fetch even when body is None: {result:?}"
    );
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["id"], 8001);
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
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut story = seed_minimal(8002, BoardState::Ready);
    story.body = Some("# real body\n\ncached body content from prior hydrate".to_string());
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let seed = SeededStore::new(vec![], vec![], vec![]);
    seed.bind_refresh(&refresh);
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![story])));
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(8002));
    let result = handle_get_story(&args, Some(&refresh)).await;
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
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let seed = SeededStore::new(vec![], vec![], vec![]);
    seed.bind_refresh(&refresh);
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_minimal(
        8003,
        BoardState::InProgress(Stage::Dev.id()),
    )])));
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(8003));
    let result = handle_get_story(&args, Some(&refresh)).await;
    assert!(
        result.is_ok(),
        "get_story does not invoke adapter.fetch_body even though body is missing + adapter would fail: {result:?}"
    );
}

#[tokio::test]
async fn select_stories_to_hydrate_picks_active_states_with_missing_body() {
    use crate::bridge::select_stories_to_hydrate;
    let mut active = seed_minimal(100, BoardState::Ready);
    active.body = None;
    let mut hydrated = seed_minimal(101, BoardState::InProgress(Stage::Dev.id()));
    hydrated.body = Some("already-cached".to_string());
    let mut accepted = seed_minimal(102, BoardState::Accepted);
    accepted.body = None;
    let mut icebox = seed_minimal(103, BoardState::Icebox);
    icebox.body = None;
    let mut delivered = seed_minimal(104, BoardState::Acceptance);
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
            let mut s = seed_minimal(200 + i, BoardState::Ready);
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
    let mut s = seed_minimal(300, BoardState::Ready);
    s.body = Some("already".to_string());
    let picks = select_stories_to_hydrate(&[s], 5);
    assert!(picks.is_empty(), "no missing-body stories = empty picks");
}

#[tokio::test]
async fn body_fetched_event_with_etag_stores_etag_on_canonical_story() {
    use crate::product_management::story::events::StoryEvent;
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let seed = SeededStore::new(vec![], vec![], vec![]);
    seed.bind_refresh(&orch);
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_minimal(
        9000,
        BoardState::Ready,
    )])));
    seed.dispatch(AppAction::Story(StoryAction::Event(
        StoryEvent::BodyFetched {
            meta: crate::sync::cache_fill_meta(),
            id: StoryId::new(9000),
            body: "hydrated body content".to_string(),
            etag: Some("W/\"deadbeef\"".to_string()),
        },
    )));
    let story = orch.story(9000).expect("story present");
    assert_eq!(story.body.as_deref(), Some("hydrated body content"));
    assert_eq!(story.body_etag.as_deref(), Some("W/\"deadbeef\""));
}

#[tokio::test]
async fn local_body_mutate_invalidates_prior_etag() {
    use crate::product_management::story::events::{EventMeta, StoryEvent};
    use crate::product_management::story::peer::PeerId;
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch);
    let mut seed = seed_minimal(9100, BoardState::Ready);
    seed.body = Some("v1".to_string());
    seed.body_etag = Some("W/\"etag-v1\"".to_string());
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed.clone()])));
    let mut updated = seed;
    updated.body = Some("v2".to_string());
    handle.dispatch(AppAction::Story(StoryAction::Event(
        StoryEvent::StoryRefreshed {
            meta: EventMeta {
                actor: "test".to_string(),
                ts: Utc::now(),
                peer_id: PeerId::local(),
            },
            id: StoryId::new(9100),
            story: Box::new(updated),
        },
    )));
    let after = orch.story(9100).expect("story present");
    assert_eq!(after.body.as_deref(), Some("v2"));
    assert_eq!(
        after.body_etag, None,
        "local mutate cleared prior etag so next conditional poll re-fetches"
    );
}

#[tokio::test]
async fn labels_refreshed_adopts_adapter_decoded_fields_so_selectors_read_fresh() {
    use crate::product_management::story::events::{EventMeta, StoryEvent};
    use crate::product_management::story::peer::PeerId;
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch);
    let seed = seed_minimal(9200, BoardState::InProgress(Stage::Dev.id()));
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed])));
    let labels = vec!["in progress".to_string(), "ready-order.300".to_string()];
    handle.dispatch(AppAction::Story(StoryAction::Event(
        StoryEvent::LabelsRefreshed {
            meta: EventMeta {
                actor: "test".to_string(),
                ts: Utc::now(),
                peer_id: PeerId::local(),
            },
            id: StoryId::new(9200),
            fields: crate::product_management::board::adapters::github::project_labels(&labels),
        },
    )));
    let after = orch.story(9200).expect("story present");
    assert_eq!(
        after.ready_order(),
        Some(300),
        "a label refresh must adopt the adapter-decoded typed fields so derived selectors read fresh values, not the stale pre-refresh story; got {:?}",
        after.ready_order()
    );
}

#[tokio::test]
async fn fetch_body_conditional_dispatches_event_with_etag_on_modified() {
    use crate::product_management::board::sync::ConditionalBody;
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_minimal(9001, BoardState::Ready)]);
    adapter.seed_body(9001, "real body".into());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();
    let result = orch
        .fetch_body_conditional(9001, None)
        .await
        .expect("conditional fetch ok");
    assert!(matches!(result, ConditionalBody::Modified { .. }));
    assert_eq!(orch.story(9001).unwrap().body.as_deref(), Some("real body"));
}

#[tokio::test]
async fn select_stories_to_revalidate_picks_already_hydrated_active_stories() {
    use crate::bridge::select_stories_to_revalidate;
    let mut active_with_body = seed_minimal(700, BoardState::Ready);
    active_with_body.body = Some("body".to_string());
    active_with_body.body_etag = Some("etag-700".to_string());
    let mut active_no_body = seed_minimal(701, BoardState::InProgress(Stage::Dev.id()));
    active_no_body.body = None;
    let mut accepted_with_body = seed_minimal(702, BoardState::Accepted);
    accepted_with_body.body = Some("body".to_string());
    let mut icebox_with_body = seed_minimal(703, BoardState::Icebox);
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
            let mut s = seed_minimal(800 + i, BoardState::Ready);
            s.body = Some("body".to_string());
            s
        })
        .collect();
    let picks = select_stories_to_revalidate(&stories, 2);
    assert_eq!(picks.len(), 2, "limit caps revalidation volume");
}

#[tokio::test]
async fn get_story_returns_gate_comments_hydrated_flag_in_response() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let seed = SeededStore::new(vec![], vec![], vec![]);
    seed.bind_refresh(&refresh);
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_minimal(
        8004,
        BoardState::Ready,
    )])));
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(8004));
    let result = handle_get_story(&args, Some(&refresh)).await;
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
