#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState, StoryType};
use crate::board::events::{BoardEvent, EventMeta};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::peer::PeerId;
use crate::board::store::BoardStore;

#[tokio::test]
async fn ingest_clears_recovering_from_corruption_flag() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator: Arc<dyn crate::board::RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.mark_recovering_from_corruption();
    assert!(
        orchestrator.snapshot_status().recovering_from_corruption,
        "flag set after mark_recovering_from_corruption"
    );
    orchestrator.refresh_all().await.unwrap();
    assert!(
        !orchestrator.snapshot_status().recovering_from_corruption,
        "successful ingest clears recovering flag"
    );
}

#[tokio::test]
async fn failed_ingest_keeps_recovering_from_corruption_flag() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let orchestrator: Arc<dyn crate::board::RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.mark_recovering_from_corruption();
    let _ = orchestrator.refresh_all().await;
    assert!(
        orchestrator.snapshot_status().recovering_from_corruption,
        "failed ingest preserves recovering flag for next attempt"
    );
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
            story_type: StoryType::Feature,
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
