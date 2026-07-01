#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::events::{EventMeta, StoryEvent};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::store::{BoardStore, StoreChange};
use crate::product_management::story::{BoardState, Stage, Story, StoryId};

#[tokio::test]
async fn refresh_story_pulls_single_story_from_adapter_into_store() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(42, BoardState::Ready)]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    assert!(
        store.story(StoryId::new(42)).is_none(),
        "store starts empty"
    );
    let result = orchestrator.refresh_story(42).await.unwrap();
    assert!(result.is_some());
    assert_eq!(
        store.story(StoryId::new(42)).unwrap().state,
        BoardState::Ready
    );
    assert!(orchestrator.snapshot_status().last_ingest_at.is_some());
}

#[tokio::test]
async fn refresh_story_returns_none_when_story_missing_at_source() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    let result = orchestrator.refresh_story(999).await.unwrap();
    assert!(result.is_none());
    assert!(store.story(StoryId::new(999)).is_none());
}

#[tokio::test]
async fn refresh_story_fails_fast_when_source_unreachable() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(1, BoardState::Ready)]);
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    let result = orchestrator.refresh_story(1).await;
    assert!(result.is_err());
    assert!(
        orchestrator.snapshot_status().last_error.is_some(),
        "source-unreachable records error"
    );
}

#[tokio::test]
async fn refresh_all_is_alias_for_ingest_once() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![
        seed_story(1, BoardState::Ready),
        seed_story(2, BoardState::Delivered),
    ]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.refresh_all().await.unwrap();
    assert_eq!(store.len(), 2);
}

#[tokio::test]
async fn end_to_end_seeded_adapter_through_mutate_and_drain() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(42, BoardState::Ready)]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(
        store.story(StoryId::new(42)).unwrap().state,
        BoardState::Ready
    );
    orchestrator
        .mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(42),
            new_state: BoardState::InProgress(Stage::Dev),
        })
        .unwrap();
    assert_eq!(
        store.story(StoryId::new(42)).unwrap().state,
        BoardState::InProgress(Stage::Dev)
    );
    orchestrator.drain_once().await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    match &pushed[0] {
        StoryEvent::StateTransitioned { new_state, .. } => {
            assert_eq!(*new_state, BoardState::InProgress(Stage::Dev));
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn fetch_body_caches_the_body_on_the_story_and_fires_event() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(7, BoardState::Ready)]);
    adapter.seed_body(7, "the body text".into());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(store.story(StoryId::new(7)).unwrap().body, None);

    let mut rx = store.subscribe();
    let returned = orchestrator.fetch_body(7).await.unwrap();
    assert_eq!(returned, "the body text");
    assert_eq!(
        store.story(StoryId::new(7)).unwrap().body.as_deref(),
        Some("the body text")
    );
    let event = rx.recv().await.unwrap();
    assert!(
        matches!(event, StoreChange::Event(StoryEvent::BodyFetched { id, .. }) if id == StoryId::new(7))
    );
}

#[tokio::test]
async fn fetch_body_records_error_when_source_fails() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(7, BoardState::Ready)]);
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    let err = orchestrator.fetch_body(7).await.unwrap_err();
    assert!(matches!(
        err,
        crate::product_management::board::SyncError::SourceUnreachable(_)
    ));
    let status = orchestrator.snapshot_status();
    assert!(status.last_error.unwrap().contains("fetch_body"));
}

#[tokio::test]
async fn fetch_free_comments_caches_comments_and_fires_event() {
    use crate::product_management::story::FreeComment;
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(9, BoardState::Ready)]);
    let seeded = vec![FreeComment {
        body: "hi".into(),
        author: "owner".into(),
        ts: Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap(),
    }];
    adapter.seed_free_comments(9, seeded.clone());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(store.story(StoryId::new(9)).unwrap().free_comments, None);

    let mut rx = store.subscribe();
    let returned = orchestrator.fetch_free_comments(9).await.unwrap();
    assert_eq!(returned, seeded);
    assert_eq!(
        store.story(StoryId::new(9)).unwrap().free_comments.unwrap(),
        seeded
    );
    let event = rx.recv().await.unwrap();
    assert!(matches!(
        event,
        StoreChange::Event(StoryEvent::FreeCommentsFetched { id, .. }) if id == StoryId::new(9)
    ));
}

#[tokio::test]
async fn fetch_gate_comments_caches_comments_and_fires_event() {
    use crate::product_management::story::{Gate, GateComment};
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(11, BoardState::Ready)]);
    let seeded = vec![GateComment {
        gate: Gate::Pickup,
        ts: Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap(),
        actor: "dev".into(),
        payload: Some("paraphrase".into()),
    }];
    adapter.seed_gate_comments(11, seeded.clone());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(store.story(StoryId::new(11)).unwrap().gate_comments, None);

    let mut rx = store.subscribe();
    let returned = orchestrator.fetch_gate_comments(11).await.unwrap();
    assert_eq!(returned, seeded);
    assert_eq!(
        store
            .story(StoryId::new(11))
            .unwrap()
            .gate_comments
            .unwrap(),
        seeded
    );
    let event = rx.recv().await.unwrap();
    assert!(matches!(
        event,
        StoreChange::Event(StoryEvent::GateCommentsFetched { id, .. }) if id == StoryId::new(11)
    ));
}

#[tokio::test]
async fn fetch_gate_comments_records_error_when_source_fails() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(11, BoardState::Ready)]);
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    let err = orchestrator.fetch_gate_comments(11).await.unwrap_err();
    assert!(matches!(
        err,
        crate::product_management::board::SyncError::SourceUnreachable(_)
    ));
    assert!(
        orchestrator
            .snapshot_status()
            .last_error
            .unwrap()
            .contains("fetch_gate_comments")
    );
}
