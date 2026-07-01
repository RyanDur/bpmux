#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::events::{EventMeta, StoryEvent};
use crate::product_management::board::orchestrator::{
    ConflictKind, QueuedEvent, SyncOrchestrator, classify_conflict, load_outbox_from_disk,
};
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{BoardState, Stage, Story, StoryId};

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 5, 5, 22, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn seed_story(id: u64, state: BoardState) -> Story {
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
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(500, BoardState::Ready)]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outbox.json");
    let orch =
        SyncOrchestrator::with_outbox_path(Arc::clone(&store), Arc::clone(&adapter), path.clone())
            .unwrap();
    orch.refresh_all().await.unwrap();

    orch.mutate(StoryEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(500),
        new_state: BoardState::InProgress(Stage::Dev),
    })
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
        BoardState::Ready,
        "pre_state must reflect the pre-mutation state, not post"
    );
    assert!(matches!(
        entry.event,
        StoryEvent::StateTransitioned {
            new_state: BoardState::InProgress(Stage::Dev),
            ..
        }
    ));
}

#[tokio::test]
async fn mutate_pre_state_none_when_story_not_in_store_at_queue_time() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outbox.json");
    let orch =
        SyncOrchestrator::with_outbox_path(Arc::clone(&store), Arc::clone(&adapter), path.clone())
            .unwrap();

    orch.mutate(StoryEvent::TagAdded {
        meta: meta(),
        id: StoryId::new(999),
        label: "blocked".to_string(),
    })
    .unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(
        entries.front().unwrap().pre_state.is_none(),
        "pre_state must be None when canonical store doesn't have the story at queue-time"
    );
}

#[tokio::test]
async fn outbox_schema_back_compat_reads_legacy_bare_boardevent_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy-outbox.json");
    let legacy_event = StoryEvent::TagAdded {
        meta: meta(),
        id: StoryId::new(42),
        label: "ready".to_string(),
    };
    let legacy_payload = serde_json::to_vec(&vec![legacy_event]).unwrap();
    std::fs::write(&path, legacy_payload).unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = entries.front().unwrap();
    assert!(matches!(
        entry.event,
        StoryEvent::TagAdded { id, .. } if id == StoryId::new(42)
    ));
    assert!(
        entry.pre_state.is_none(),
        "legacy entries map to QueuedEvent with pre_state: None"
    );
}

#[tokio::test]
async fn outbox_round_trips_new_shape_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new-outbox.json");
    let pre = seed_story(7, BoardState::Delivered);
    let queued = QueuedEvent::new(
        StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(7),
            new_state: BoardState::Accepted,
        },
        Some(pre.clone()),
    );
    let payload = serde_json::to_vec(&vec![queued]).unwrap();
    std::fs::write(&path, payload).unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = entries.front().unwrap();
    let restored_pre = entry.pre_state.as_ref().unwrap();
    assert_eq!(restored_pre.id, pre.id);
    assert_eq!(restored_pre.state, BoardState::Delivered);
}

#[test]
fn classify_conflict_returns_none_for_slice_a_pending_real_classifier() {
    let pre = seed_story(1, BoardState::Ready);
    let cur = seed_story(1, BoardState::InProgress(Stage::Dev));
    let event = StoryEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(1),
        new_state: BoardState::InProgress(Stage::Dev),
    };
    assert_eq!(classify_conflict(Some(&pre), &event, Some(&cur)), None);
}

#[test]
fn conflict_kind_serde_round_trip() {
    let kind = ConflictKind::IdempotentSkip;
    let json = serde_json::to_string(&kind).unwrap();
    let back: ConflictKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, kind);
}

#[tokio::test]
async fn drain_resets_conflicts_pending_to_zero_in_slice_a() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(800, BoardState::Ready)]);
    let orch: Arc<dyn crate::product_management::board::RefreshDriver> = Arc::new(
        SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter)),
    );
    orch.refresh_all().await.unwrap();

    orch.mutate_sync(StoryEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(800),
        new_state: BoardState::InProgress(Stage::Dev),
    })
    .unwrap();

    let _ = orch.drain_pending().await.unwrap();
    assert_eq!(orch.snapshot_status().conflicts_pending, 0);
}
