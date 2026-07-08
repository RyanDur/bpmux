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
use crate::store::{AppAction, Store, SyncAction};
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
async fn start_update_github_burst_pushes_every_queued_event_exactly_once() {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    std::mem::forget(dir);

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let refresh: Arc<dyn RefreshDriver> = orch.clone();

    orch.mutate(tag_added(5001, "one")).unwrap();
    orch.mutate(tag_added(5002, "two")).unwrap();
    orch.mutate(tag_added(5003, "three")).unwrap();

    let store: Arc<Store> = Arc::new(
        Store::new(hub, None, CiStateStore::new(), CancellationToken::new()).with_refresh(
            refresh,
            Duration::from_secs(60),
            Duration::from_secs(15),
        ),
    );

    let mutation_id = crate::sync::MutationId::default();
    for _ in 0..3 {
        store
            .dispatch(AppAction::Sync(SyncAction::StartUpdateGithub {
                mutation_id,
            }))
            .await;
    }

    crate::store::test_support::eventually(
        "the StartUpdateGithub burst single-flights one drain pass that pushes all three queued writes; a second pass over the emptied queue never re-pushes any of them",
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
    assert!(
        orch.snapshot_pending_for_test().is_empty(),
        "the queue is empty once the pass completes"
    );
}
