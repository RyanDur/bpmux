#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use crate::bridge::tools::handle_get_story;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, SeededStore};
use crate::sync::SyncOrchestrator;
use product_management::iteration::{Iteration, IterationId};
use product_management::story::slice::StoryAction;
use product_management::story::{Story, StoryId, Work};

fn seed_minimal(id: u64, state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("[Feature] target-{id}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn current_iteration_of(members: &[u64]) -> Vec<Iteration> {
    vec![Iteration {
        id: IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: members.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }]
}

#[tokio::test]
async fn get_story_canonical_only_returns_placeholder_when_body_missing() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let seed = SeededStore::new(vec![], vec![], vec![]);
    let refresh: Arc<SyncOrchestrator> = seed.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_minimal(
        8001,
        Work::NotStarted,
    )])));
    let select = seed.select();
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(8001));
    let result = handle_get_story(&args, Some(&refresh), &select).await;
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
    let mut story = seed_minimal(8002, Work::NotStarted);
    story.body = Some("# real body\n\ncached body content from prior hydrate".to_string());
    let seed = SeededStore::new(vec![], vec![], vec![]);
    let refresh: Arc<SyncOrchestrator> = seed.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![story])));
    let select = seed.select();
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(8002));
    let result = handle_get_story(&args, Some(&refresh), &select).await;
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
    let seed = SeededStore::new(vec![], vec![], vec![]);
    let refresh: Arc<SyncOrchestrator> = seed.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_minimal(
        8003,
        Work::InProgress,
    )])));
    let select = seed.select();
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(8003));
    let result = handle_get_story(&args, Some(&refresh), &select).await;
    assert!(
        result.is_ok(),
        "get_story does not invoke adapter.fetch_body even though body is missing + adapter would fail: {result:?}"
    );
}

#[tokio::test]
async fn select_stories_to_hydrate_picks_active_states_with_missing_body() {
    use crate::bridge::select_stories_to_hydrate;
    let mut active = seed_minimal(100, Work::NotStarted);
    active.body = None;
    let mut hydrated = seed_minimal(101, Work::InProgress);
    hydrated.body = Some("already-cached".to_string());
    let mut accepted = seed_minimal(102, Work::Accepted);
    accepted.body = None;
    let mut icebox = seed_minimal(103, Work::NotStarted);
    icebox.body = None;
    let mut delivered = seed_minimal(104, Work::Acceptance);
    delivered.body = None;
    let stories = vec![active, hydrated, accepted, icebox, delivered];
    let picks = select_stories_to_hydrate(&stories, 10, &current_iteration_of(&[100]));
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
            let mut s = seed_minimal(200 + i, Work::NotStarted);
            s.body = None;
            s
        })
        .collect();
    let members: Vec<u64> = (0..10).map(|i| 200 + i).collect();
    let picks = select_stories_to_hydrate(&stories, 3, &current_iteration_of(&members));
    assert_eq!(picks.len(), 3, "limit caps per-tick fetch volume");
}

#[tokio::test]
async fn select_stories_to_hydrate_returns_empty_when_all_hydrated() {
    use crate::bridge::select_stories_to_hydrate;
    let mut s = seed_minimal(300, Work::NotStarted);
    s.body = Some("already".to_string());
    let picks = select_stories_to_hydrate(&[s], 5, &current_iteration_of(&[300]));
    assert!(picks.is_empty(), "no missing-body stories = empty picks");
}

#[tokio::test]
async fn body_fetched_event_with_etag_stores_etag_on_canonical_story() {
    use product_management::story::events::{StoryEvent, StoryEventKind};
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seed = SeededStore::new(vec![], vec![], vec![]);
    let _orch: Arc<SyncOrchestrator> = seed.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_minimal(
        9000,
        Work::NotStarted,
    )])));
    seed.dispatch(AppAction::Story(StoryAction::Event(StoryEvent::new(
        crate::sync::cache_fill_meta(),
        StoryId::new(9000),
        StoryEventKind::BodyFetched {
            body: "hydrated body content".to_string(),
            etag: Some("W/\"deadbeef\"".to_string()),
        },
    ))));
    let story = seed
        .select()
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(9000)))
        .expect("story present");
    assert_eq!(story.body.as_deref(), Some("hydrated body content"));
    assert_eq!(story.body_etag.as_deref(), Some("W/\"deadbeef\""));
}

#[tokio::test]
async fn local_body_mutate_invalidates_prior_etag() {
    use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let handle = SeededStore::new(vec![], vec![], vec![]);
    let _orch: Arc<SyncOrchestrator> = handle.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    let select = handle.select();
    let mut seed = seed_minimal(9100, Work::NotStarted);
    seed.body = Some("v1".to_string());
    seed.body_etag = Some("W/\"etag-v1\"".to_string());
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed.clone()])));
    let mut updated = seed;
    updated.body = Some("v2".to_string());
    handle.dispatch(AppAction::Story(StoryAction::Event(StoryEvent::new(
        EventMeta {
            actor: "test".to_string(),
            actor_id: None,
            ts: Utc::now(),
        },
        StoryId::new(9100),
        StoryEventKind::StoryRefreshed {
            story: Box::new(updated),
        },
    ))));
    let after = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(9100)))
        .expect("story present");
    assert_eq!(after.body.as_deref(), Some("v2"));
    assert_eq!(
        after.body_etag, None,
        "local mutate cleared prior etag so next conditional poll re-fetches"
    );
}

#[tokio::test]
async fn labels_refreshed_adopts_adapter_decoded_fields_so_selectors_read_fresh() {
    use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let handle = SeededStore::new(vec![], vec![], vec![]);
    let _orch: Arc<SyncOrchestrator> = handle.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    let select = handle.select();
    let seed = seed_minimal(9200, Work::InProgress);
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed])));
    let labels = vec!["in progress".to_string(), "awaiting owner".to_string()];
    handle.dispatch(AppAction::Story(StoryAction::Event(StoryEvent::new(
        EventMeta {
            actor: "test".to_string(),
            actor_id: None,
            ts: Utc::now(),
        },
        StoryId::new(9200),
        StoryEventKind::LabelsRefreshed {
            fields: source_codec::mapper::project_labels(&labels),
        },
    ))));
    let after = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(9200)))
        .expect("story present");
    assert_eq!(
        after.acceptor(),
        Some(product_management::story::Acceptor::Owner),
        "a label refresh must adopt the adapter-decoded typed fields so derived selectors read fresh values, not the stale pre-refresh story; got {:?}",
        after.acceptor()
    );
}

#[tokio::test]
async fn fetch_body_conditional_dispatches_event_with_etag_on_modified() {
    use crate::sync::ConditionalBody;
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_minimal(9001, Work::NotStarted)]);
    adapter.seed_body(9001, "real body".into());
    let (orch, select) = crate::store::build_seeded_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        vec![],
        vec![],
        vec![],
    );
    orch.refresh_all().await.unwrap();
    let result = orch
        .fetch_body_conditional(9001, None)
        .await
        .expect("conditional fetch ok");
    assert!(matches!(result, ConditionalBody::Modified { .. }));
    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(9001)
            ))
            .unwrap()
            .body
            .as_deref(),
        Some("real body")
    );
}

#[tokio::test]
async fn select_stories_to_revalidate_picks_already_hydrated_active_stories() {
    use crate::bridge::select_stories_to_revalidate;
    let mut active_with_body = seed_minimal(700, Work::NotStarted);
    active_with_body.body = Some("body".to_string());
    active_with_body.body_etag = Some("etag-700".to_string());
    let mut active_no_body = seed_minimal(701, Work::InProgress);
    active_no_body.body = None;
    let mut accepted_with_body = seed_minimal(702, Work::Accepted);
    accepted_with_body.body = Some("body".to_string());
    let mut icebox_with_body = seed_minimal(703, Work::NotStarted);
    icebox_with_body.body = Some("body".to_string());
    let stories = vec![
        active_with_body,
        active_no_body,
        accepted_with_body,
        icebox_with_body,
    ];
    let picks = select_stories_to_revalidate(&stories, 10, &current_iteration_of(&[700]));
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
            let mut s = seed_minimal(800 + i, Work::NotStarted);
            s.body = Some("body".to_string());
            s
        })
        .collect();
    let members: Vec<u64> = (0..10).map(|i| 800 + i).collect();
    let picks = select_stories_to_revalidate(&stories, 2, &current_iteration_of(&members));
    assert_eq!(picks.len(), 2, "limit caps revalidation volume");
}

#[tokio::test]
async fn get_story_returns_hydrated_flag_in_response() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seed = SeededStore::new(vec![], current_iteration_of(&[8004]), vec![]);
    let refresh: Arc<SyncOrchestrator> = seed.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    seed.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_minimal(
        8004,
        Work::NotStarted,
    )])));
    let select = seed.select();
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(8004));
    let result = handle_get_story(&args, Some(&refresh), &select).await;
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
