#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Stage, Story, StoryId, StoryKind, Work};
use crate::store::{AppAction, SeededStore};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;

#[tokio::test]
async fn ingest_once_populates_store_from_adapter() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![
        seed_story(1, Work::NotStarted),
        seed_story(2, Work::Acceptance),
    ]);
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(orchestrator.all_stories().len(), 2);
    assert!(orchestrator.story(1).is_some());
    assert!(orchestrator.story(2).is_some());
    assert!(orchestrator.snapshot_status().last_ingest_at.is_some());
}

#[tokio::test]
async fn ingest_once_records_error_on_source_failure() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(adapter);
    let result = orchestrator.ingest_once().await;
    assert!(result.is_err());
    let status = orchestrator.snapshot_status();
    assert!(status.last_error.is_some());
    assert!(status.last_ingest_at.is_none());
}

#[tokio::test]
async fn mutate_applies_event_locally_and_enqueues_for_push() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(100),
            StoryEventKind::StoryCreated {
                title: "test".to_string(),
                body: "b".to_string(),
                initial_state: Work::NotStarted,
                story_type: StoryKind::Feature,
            },
        ))
        .unwrap();
    assert_eq!(orchestrator.all_stories().len(), 1);
    assert_eq!(orchestrator.story(100).unwrap().title, "test");
    assert_eq!(orchestrator.snapshot_status().pending_events, 1);
    assert!(
        adapter.pushed_events().is_empty(),
        "push is async; drain_once fires it"
    );
}

#[tokio::test]
async fn drain_once_pushes_pending_events_to_adapter() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
        1,
        Work::NotStarted,
    )])));
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(1),
            StoryEventKind::StateTransitioned {
                new_state: Work::Acceptance,
                placement: None,
            },
        ))
        .unwrap();
    let drained = orchestrator.drain_once().await.unwrap();
    assert_eq!(drained, 1);
    assert_eq!(adapter.pushed_events().len(), 1);
    assert_eq!(orchestrator.snapshot_status().pending_events, 0);
    assert!(orchestrator.snapshot_status().last_push_at.is_some());
}

#[tokio::test]
async fn drain_once_requeues_event_on_adapter_failure() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
        1,
        Work::NotStarted,
    )])));
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(1),
            StoryEventKind::StateTransitioned {
                new_state: Work::Acceptance,
                placement: None,
            },
        ))
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
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![
        seed_story(1, Work::NotStarted),
        seed_story(2, Work::NotStarted),
    ])));
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(1),
            StoryEventKind::StateTransitioned {
                new_state: Work::Acceptance,
                placement: None,
            },
        ))
        .unwrap();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(2),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
    assert_eq!(orchestrator.snapshot_status().pending_events, 2);
    let drained = orchestrator.drain_once().await.unwrap();
    assert_eq!(drained, 2);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert_eq!(pushed[0].story_id(), StoryId::new(1));
    assert_eq!(pushed[1].story_id(), StoryId::new(2));
}

#[tokio::test]
async fn ingest_preserves_an_in_flight_reorder_against_a_pre_drain_refresh() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = seed_story(1687, Work::NotStarted);
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(1687, vec!["iteration-Pegasus.125".to_string()]);
    let orchestrator = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);

    orchestrator.ingest_once().await.unwrap();
    assert_eq!(
        orchestrator
            .story(1687)
            .unwrap()
            .iteration_slot()
            .map(|slot| slot.position),
        Some(125),
        "baseline: story starts slotted at the old position"
    );

    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(1687),
            StoryEventKind::IterationSlotSet {
                name: "Pegasus".to_string(),
                position: 106,
            },
        ))
        .unwrap();
    assert_eq!(
        orchestrator
            .story(1687)
            .unwrap()
            .iteration_slot()
            .map(|slot| slot.position),
        Some(106),
        "optimistic reorder moves the story to the new position"
    );

    orchestrator.ingest_once().await.unwrap();

    assert_eq!(
        orchestrator
            .story(1687)
            .unwrap()
            .iteration_slot()
            .map(|slot| slot.position),
        Some(106),
        "a refresh that reads the source before the reorder's write has drained must NOT clobber the un-acknowledged optimistic position"
    );
}
