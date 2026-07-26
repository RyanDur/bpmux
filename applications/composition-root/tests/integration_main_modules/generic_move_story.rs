#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use hub::Hub;
use integrations::in_memory::InMemorySourceAdapter;
use product_management::story::events::EventMeta;
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{Story, StoryId, Work};
use sync::SyncOrchestrator;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn start_story_deliver_story_universal_reap_emits_8_strips_each() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(909),
        "slice 1 reap parity".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 28, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);

    let hub_dir = tempfile::tempdir().expect("hub dir");
    let (wake_tx, _) = broadcast::channel(64);
    let hub = Arc::new(Hub::new(
        hub_dir.path().to_path_buf(),
        "test".to_string(),
        wake_tx,
    ));
    let app_store = Arc::new(server::store_wiring::build_store(
        hub.clone(),
        storage::open(&hub.dir),
        None,
        CancellationToken::new(),
    ));
    let orch: Arc<SyncOrchestrator> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        app_store.selector(),
        app_store.dispatcher(),
    ));
    orch.refresh_all().await.unwrap();

    let meta = EventMeta {
        actor: "dev".into(),
        actor_id: None,
        ts: Utc::now(),
    };

    for stale in [
        "icebox",
        "ready",
        "desk-check",
        "delivered",
        "ready for accept",
        "needs fixes",
        "rejected",
        "accepted",
    ] {
        orch.mutate(StoryEvent::new(
            meta.clone(),
            StoryId::new(909),
            StoryEventKind::TagRemoved {
                label: stale.to_string(),
            },
        ))
        .unwrap();
    }
    let drained = orch.drain_pass().await.unwrap();
    assert_eq!(
        drained, 8,
        "8 strips expected — one per non-target ALL_STATE_LABELS entry"
    );

    let pushed = adapter.pushed_events();
    let strip_count = pushed
        .iter()
        .filter(
            |e| matches!(&e.kind, StoryEventKind::TagRemoved { .. } if e.id == StoryId::new(909)),
        )
        .count();
    assert_eq!(strip_count, 8);
}
