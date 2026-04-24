#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::adapters::in_memory::InMemorySourceAdapter;
use super::domain::{Story, StoryState};
use super::events::{BoardEvent, EventMeta};
use super::orchestrator::SyncOrchestrator;
use super::peer::PeerId;
use super::store::BoardStore;

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 4, 24, 16, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn seed_story(number: u64, state: StoryState) -> Story {
    Story::new_minimal(
        number,
        format!("title-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[tokio::test]
async fn ingest_once_populates_store_from_adapter() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![
        seed_story(1, StoryState::Ready),
        seed_story(2, StoryState::Delivered),
    ]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(store.len(), 2);
    assert!(store.story(1).is_some());
    assert!(store.story(2).is_some());
    assert!(orchestrator.snapshot_status().last_ingest_at.is_some());
}

#[tokio::test]
async fn ingest_once_records_error_on_source_failure() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    let result = orchestrator.ingest_once().await;
    assert!(result.is_err());
    let status = orchestrator.snapshot_status();
    assert!(status.last_error.is_some());
    assert!(status.last_ingest_at.is_none());
}

#[tokio::test]
async fn mutate_applies_event_locally_and_enqueues_for_push() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator
        .mutate(BoardEvent::StoryCreated {
            meta: meta(),
            number: 100,
            title: "test".to_string(),
            body: "b".to_string(),
            initial_state: StoryState::Ready,
        })
        .unwrap();
    assert_eq!(store.len(), 1);
    assert_eq!(store.story(100).unwrap().title, "test");
    assert_eq!(orchestrator.snapshot_status().pending_events, 1);
    assert!(
        adapter.pushed_events().is_empty(),
        "push is async; drain_once fires it"
    );
}

#[tokio::test]
async fn drain_once_pushes_pending_events_to_adapter() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    store.seed_all(vec![seed_story(1, StoryState::Ready)]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator
        .mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 1,
            new_state: StoryState::Delivered,
        })
        .unwrap();
    let drained = orchestrator.drain_once().await.unwrap();
    assert_eq!(drained, 1);
    assert_eq!(adapter.pushed_events().len(), 1);
    assert_eq!(orchestrator.snapshot_status().pending_events, 0);
    assert!(orchestrator.snapshot_status().last_push_at.is_some());
}

#[tokio::test]
async fn drain_once_requeues_event_on_adapter_failure() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    store.seed_all(vec![seed_story(1, StoryState::Ready)]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator
        .mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 1,
            new_state: StoryState::Delivered,
        })
        .unwrap();
    adapter.set_push_failures(true);
    let result = orchestrator.drain_once().await;
    assert!(result.is_err(), "drain should surface failure count");
    assert!(
        adapter.pushed_events().is_empty(),
        "failed push does not record"
    );
    let status = orchestrator.snapshot_status();
    assert!(
        status.last_error.is_some(),
        "error recorded on drain failure"
    );
    adapter.set_push_failures(false);
    let drained = orchestrator.drain_once().await.unwrap();
    assert_eq!(drained, 1, "requeued event drains on retry");
}

#[tokio::test]
async fn drain_once_handles_multiple_events_in_order() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    store.seed_all(vec![
        seed_story(1, StoryState::Ready),
        seed_story(2, StoryState::Ready),
    ]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator
        .mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 1,
            new_state: StoryState::Delivered,
        })
        .unwrap();
    orchestrator
        .mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 2,
            new_state: StoryState::InProgress,
        })
        .unwrap();
    assert_eq!(orchestrator.snapshot_status().pending_events, 2);
    let drained = orchestrator.drain_once().await.unwrap();
    assert_eq!(drained, 2);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert_eq!(pushed[0].story_number(), 1);
    assert_eq!(pushed[1].story_number(), 2);
}

#[tokio::test]
async fn end_to_end_seeded_adapter_through_mutate_and_drain() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(42, StoryState::Ready)]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(store.story(42).unwrap().state, StoryState::Ready);
    orchestrator
        .mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 42,
            new_state: StoryState::InProgress,
        })
        .unwrap();
    assert_eq!(store.story(42).unwrap().state, StoryState::InProgress);
    orchestrator.drain_once().await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    match &pushed[0] {
        BoardEvent::StateTransitioned { new_state, .. } => {
            assert_eq!(*new_state, StoryState::InProgress);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}
