#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{Stage, Story, StoryId, Work};
use crate::store::SeededStore;
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;

#[tokio::test]
async fn refresh_story_pulls_single_story_from_adapter_into_store() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(42, Work::NotStarted)]);
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    assert!(orchestrator.story(42).is_none(), "store starts empty");
    let result = orchestrator.refresh_story(42).await.unwrap();
    assert!(result.is_some());
    assert_eq!(orchestrator.story(42).unwrap().state, Work::NotStarted);
    assert!(orchestrator.snapshot_status().last_ingest_at.is_some());
}

#[tokio::test]
async fn refresh_story_returns_none_when_story_missing_at_source() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let result = orchestrator.refresh_story(999).await.unwrap();
    assert!(result.is_none());
    assert!(orchestrator.story(999).is_none());
}

#[tokio::test]
async fn refresh_story_fails_fast_when_source_unreachable() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(1, Work::NotStarted)]);
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(adapter);
    let result = orchestrator.refresh_story(1).await;
    assert!(result.is_err());
    assert!(
        orchestrator.snapshot_status().last_error.is_some(),
        "source-unreachable records error"
    );
}

#[tokio::test]
async fn refresh_all_is_alias_for_ingest_once() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![
        seed_story(1, Work::NotStarted),
        seed_story(2, Work::Acceptance),
    ]);
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator.refresh_all().await.unwrap();
    assert_eq!(orchestrator.all_stories().len(), 2);
}

#[tokio::test]
async fn end_to_end_seeded_adapter_through_mutate_and_drain() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(42, Work::NotStarted)]);
    let orchestrator = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(orchestrator.story(42).unwrap().state, Work::NotStarted);
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(42),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
    assert_eq!(orchestrator.story(42).unwrap().state, Work::InProgress);
    orchestrator.drain_pass().await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    match &pushed[0].kind {
        StoryEventKind::StateTransitioned { new_state, .. } => {
            assert_eq!(*new_state, Work::InProgress);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn fetch_body_caches_the_body_on_the_story() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(7, Work::NotStarted)]);
    adapter.seed_body(7, "the body text".into());
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(orchestrator.story(7).unwrap().body, None);

    let returned = orchestrator.fetch_body(7).await.unwrap();
    assert_eq!(returned, "the body text");
    assert_eq!(
        orchestrator.story(7).unwrap().body.as_deref(),
        Some("the body text")
    );
}

#[tokio::test]
async fn fetch_body_records_error_when_source_fails() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(7, Work::NotStarted)]);
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(adapter);
    let err = orchestrator.fetch_body(7).await.unwrap_err();
    assert!(matches!(err, crate::sync::SyncError::SourceUnreachable(_)));
    let status = orchestrator.snapshot_status();
    assert!(status.last_error.unwrap().contains("fetch_body"));
}

#[tokio::test]
async fn fetch_free_comments_caches_comments_on_the_story() {
    use crate::product_management::story::FreeComment;
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(9, Work::NotStarted)]);
    let seeded = vec![FreeComment {
        body: "hi".into(),
        author: "owner".into(),
        ts: Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap(),
    }];
    adapter.seed_free_comments(9, seeded.clone());
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(orchestrator.story(9).unwrap().free_comments, None);

    let returned = orchestrator.fetch_free_comments(9).await.unwrap();
    assert_eq!(returned, seeded);
    assert_eq!(
        orchestrator.story(9).unwrap().free_comments.unwrap(),
        seeded
    );
}

#[tokio::test]
async fn fetch_verdicts_caches_comments_on_the_story() {
    use crate::product_management::story::{CastVerdict, Verdict};
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(11, Work::NotStarted)]);
    let seeded = vec![CastVerdict {
        verdict: Verdict::Succeed {
            step: "ready".into(),
        },
        ts: Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap(),
        actor: "dev".into(),
        payload: Some("paraphrase".into()),
    }];
    adapter.seed_verdicts(11, seeded.clone());
    let orchestrator = Arc::new(SyncOrchestrator::new(adapter));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(orchestrator.story(11).unwrap().verdicts, None);

    let returned = orchestrator.fetch_verdicts(11).await.unwrap();
    assert_eq!(returned, seeded);
    assert_eq!(orchestrator.story(11).unwrap().verdicts.unwrap(), seeded);
}

#[tokio::test]
async fn fetch_verdicts_records_error_when_source_fails() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(11, Work::NotStarted)]);
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(adapter);
    let err = orchestrator.fetch_verdicts(11).await.unwrap_err();
    assert!(matches!(err, crate::sync::SyncError::SourceUnreachable(_)));
    assert!(
        orchestrator
            .snapshot_status()
            .last_error
            .unwrap()
            .contains("fetch_verdicts")
    );
}
