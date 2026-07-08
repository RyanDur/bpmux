#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::ci::CiStateStore;
use crate::hub::Hub;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::message::Message;
use crate::product_management::story::StoryId;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::storage::StorageBackend;
use crate::store::Store;
use crate::store::test_support::eventually;
use crate::sync::{RefreshDriver, SyncOrchestrator};

fn meta() -> EventMeta {
    EventMeta {
        actor: "test".to_string(),
        ts: Utc::now(),
        peer_id: PeerId::local(),
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
    let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let refresh: Arc<dyn RefreshDriver> = orch.clone();

    let store: Arc<Store> = Arc::new(
        Store::new(hub, None, CiStateStore::new(), CancellationToken::new()).with_refresh(
            refresh.clone(),
            Duration::from_secs(60),
            Duration::from_secs(15),
        ),
    );
    refresh.bind_select(store.selector());
    refresh.bind_story_dispatch(store.story_dispatcher());
    refresh.bind_board_dispatch(store.board_dispatcher());

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

    let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel::<crate::store::AppAction>();
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)).with_action_tx(action_tx));
    let refresh: Arc<dyn RefreshDriver> = orch.clone();

    let store: Arc<Store> = Arc::new(
        Store::new(hub, None, CiStateStore::new(), CancellationToken::new()).with_refresh(
            refresh.clone(),
            Duration::from_secs(60),
            Duration::from_secs(15),
        ),
    );
    refresh.bind_select(store.selector());
    refresh.bind_story_dispatch(store.story_dispatcher());
    refresh.bind_board_dispatch(store.board_dispatcher());
    store.spawn_action_drainer(action_rx);

    orch.mutate(tag_added(7001, "awaiting team")).unwrap();

    let pending = store
        .selector()
        .select(|state| crate::store::sync::pending_count(&state.sync));
    assert_eq!(
        pending, 1,
        "a mutated write is queued and selectable the instant mutate returns, before any runtime yield"
    );
}
