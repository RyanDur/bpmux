#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::hub::Hub;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::server::store_wiring::build_store_with_refresh;
use crate::store::AppAction;
use crate::store::test_support::eventually;
use crate::store::{Store, SyncHealthAction};
use source_codec::gh_label_codec::GhLabelCodec;
use source_sync::{MutationId, PendingWrite, QueuedWrite};
use storage::StorageBackend;

use message::Message;
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::{Story, StoryId, Work};

fn meta() -> EventMeta {
    EventMeta {
        actor: "test".to_string(),
        actor_id: None,
        ts: Utc::now(),
    }
}

fn tag_added(story: u64, label: &str) -> StoryEvent {
    StoryEvent::new(
        meta(),
        StoryId::new(story),
        StoryEventKind::TagAdded {
            label: label.to_string(),
        },
    )
}

fn seeded_write(story: u64, label: &str, id: u64) -> QueuedWrite {
    QueuedWrite::minted(
        MutationId(id),
        PendingWrite::from_event(&GhLabelCodec, tag_added(story, label)),
    )
}

#[tokio::test]
async fn write_queued_burst_single_flights_one_drain_that_pushes_each_write_once() {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::with_backend(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
        StorageBackend::Sqlite,
    ));
    std::mem::forget(dir);

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let (store_inner, orch) = build_store_with_refresh(
        hub,
        None,
        CancellationToken::new(),
        Vec::new(),
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        Duration::from_secs(60),
        Duration::from_secs(15),
    );
    let store: Arc<Store> = Arc::new(store_inner);
    orch.mutate(tag_added(5001, "one")).unwrap();
    orch.mutate(tag_added(5002, "two")).unwrap();
    orch.mutate(tag_added(5003, "three")).unwrap();

    eventually(
        "the WriteQueued burst single-flights drain passes that push all three queued writes; a pass over the emptied queue never re-pushes any of them",
        || adapter.pushed_events().len() == 3,
    )
    .await;

    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        3,
        "single-flight drain pushes each queued write exactly once, never more"
    );
    let mut pushed_ids: Vec<u64> = pushed.iter().map(|e| e.story_id().value()).collect();
    pushed_ids.sort_unstable();
    assert_eq!(
        pushed_ids,
        vec![5001, 5002, 5003],
        "every queued write is pushed once — no write is skipped and none is duplicated"
    );
    eventually("the queue empties once the pass completes", || {
        store.selector().select(|state| state.sync.queue.is_empty())
    })
    .await;
}

#[tokio::test]
async fn polling_liveness_facts_never_drain_a_seeded_outbox() {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::with_backend(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
        StorageBackend::Sqlite,
    ));
    std::mem::forget(dir);

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let (store_inner, _orch) = build_store_with_refresh(
        hub,
        None,
        CancellationToken::new(),
        vec![seeded_write(9101, "one", 1), seeded_write(9102, "two", 2)],
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        Duration::from_secs(60),
        Duration::from_secs(15),
    );
    let store: Arc<Store> = Arc::new(store_inner);
    store
        .dispatch(AppAction::SyncHealth(
            SyncHealthAction::IngestPollingStarted,
        ))
        .await;
    store
        .dispatch(AppAction::SyncHealth(
            SyncHealthAction::HydrationPollingStarted,
        ))
        .await;
    store
        .dispatch(AppAction::SyncHealth(
            SyncHealthAction::IngestPollingStopped,
        ))
        .await;
    store
        .dispatch(AppAction::SyncHealth(
            SyncHealthAction::HydrationPollingStopped,
        ))
        .await;

    tokio::time::sleep(Duration::from_millis(250)).await;

    assert!(
        adapter.pushed_events().is_empty(),
        "a polling-liveness fact reports effector state, not a queue change — it must never trigger a drain of the seeded outbox"
    );
    let pending = store
        .selector()
        .select(|state| crate::store::sync::pending_count(&state.sync));
    assert_eq!(
        pending, 2,
        "the seeded outbox stays queued through liveness reporting — only a real drain pass may empty it"
    );
}

#[tokio::test]
async fn a_mutated_write_is_selectable_the_moment_mutate_returns() {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::with_backend(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
        StorageBackend::Sqlite,
    ));
    std::mem::forget(dir);

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let (store_inner, orch) = build_store_with_refresh(
        hub,
        None,
        CancellationToken::new(),
        Vec::new(),
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        Duration::from_secs(60),
        Duration::from_secs(15),
    );
    let store: Arc<Store> = Arc::new(store_inner);
    orch.mutate(tag_added(7001, "awaiting team")).unwrap();

    let pending = store
        .selector()
        .select(|state| crate::store::sync::pending_count(&state.sync));
    assert_eq!(
        pending, 1,
        "a mutated write is queued and selectable the instant mutate returns, before any runtime yield"
    );
}

#[tokio::test]
async fn an_ingested_upsert_is_selectable_the_moment_refresh_returns() {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::with_backend(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
        StorageBackend::Sqlite,
    ));
    std::mem::forget(dir);

    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![Story::new_minimal(
        StoryId::new(8001),
        "an outcome".to_string(),
        Work::NotStarted,
        Utc::now(),
        "pm".to_string(),
    )]);
    adapter.seed_labels(8001, vec!["outcome".to_string()]);
    let (store_inner, orch) = build_store_with_refresh(
        hub,
        None,
        CancellationToken::new(),
        Vec::new(),
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        Duration::from_secs(60),
        Duration::from_secs(15),
    );
    let store: Arc<Store> = Arc::new(store_inner);
    orch.refresh_all().await.unwrap();

    let outcomes = store
        .selector()
        .select(|state| product_management::outcome::all(&state.outcomes).len());
    assert_eq!(
        outcomes, 1,
        "an ingested upsert is reduced into the store and selectable the instant refresh returns, before any runtime yield"
    );
}
