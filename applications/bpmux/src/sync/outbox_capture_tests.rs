#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{Story, StoryId, Work};
use crate::store::bind_seeded_refresh;
use crate::sync::RefreshDriver;
use crate::sync::{QueuedEvent, SyncOrchestrator, load_outbox_from_disk};

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 5, 5, 22, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn seed_story(id: u64, state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("title-{id}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[tokio::test]
async fn mutate_captures_pre_state_snapshot_before_local_apply() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(500, Work::NotStarted)]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outbox.json");
    let orch = Arc::new(SyncOrchestrator::with_outbox_path(adapter, path.clone()).unwrap());
    let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
    bind_seeded_refresh(&orch_dyn, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    orch.mutate(StoryEvent::new(
        meta(),
        StoryId::new(500),
        StoryEventKind::StateTransitioned {
            new_state: Work::InProgress,
            placement: None,
        },
    ))
    .unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = entries.front().unwrap();
    let pre = entry
        .pre_state
        .as_ref()
        .expect("pre_state captured at queue-time");
    assert_eq!(
        pre.state,
        Work::NotStarted,
        "pre_state must reflect the pre-mutation state, not post"
    );
    assert!(matches!(
        entry.event.kind,
        StoryEventKind::StateTransitioned {
            new_state: Work::InProgress,
            ..
        }
    ));
}

#[tokio::test]
async fn mutate_pre_state_none_when_story_not_in_store_at_queue_time() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outbox.json");
    let orch = Arc::new(SyncOrchestrator::with_outbox_path(adapter, path.clone()).unwrap());
    let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
    bind_seeded_refresh(&orch_dyn, vec![], vec![], vec![]);

    orch.mutate(StoryEvent::new(
        meta(),
        StoryId::new(999),
        StoryEventKind::TagAdded {
            label: "blocked".to_string(),
        },
    ))
    .unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(
        entries.front().unwrap().pre_state.is_none(),
        "pre_state must be None when canonical store doesn't have the story at queue-time"
    );
}

#[tokio::test]
async fn outbox_round_trips_the_only_shape_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new-outbox.json");
    let pre = seed_story(7, Work::Acceptance);
    let queued = QueuedEvent::new(
        StoryEvent::new(
            meta(),
            StoryId::new(7),
            StoryEventKind::StateTransitioned {
                new_state: Work::Accepted,
                placement: None,
            },
        ),
        Some(pre.clone()),
    );
    let payload = serde_json::to_vec(&vec![queued]).unwrap();
    std::fs::write(&path, payload).unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = entries.front().unwrap();
    let restored_pre = entry.pre_state.as_ref().unwrap();
    assert_eq!(restored_pre.id, pre.id);
    assert_eq!(restored_pre.state, Work::Acceptance);
}
