#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::adapters::in_memory::InMemorySourceAdapter;
use super::domain::{Story, StoryState, StoryType};
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

#[tokio::test]
async fn refresh_story_pulls_single_story_from_adapter_into_store() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(42, StoryState::Ready)]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    assert!(store.story(42).is_none(), "store starts empty");
    let result = orchestrator.refresh_story(42).await.unwrap();
    assert!(result.is_some());
    assert_eq!(store.story(42).unwrap().state, StoryState::Ready);
    assert!(orchestrator.snapshot_status().last_ingest_at.is_some());
}

#[tokio::test]
async fn refresh_story_returns_none_when_story_missing_at_source() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    let result = orchestrator.refresh_story(999).await.unwrap();
    assert!(result.is_none());
    assert!(store.story(999).is_none());
}

#[tokio::test]
async fn refresh_story_fails_fast_when_source_unreachable() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(1, StoryState::Ready)]);
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
        seed_story(1, StoryState::Ready),
        seed_story(2, StoryState::Delivered),
    ]);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.refresh_all().await.unwrap();
    assert_eq!(store.len(), 2);
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

#[tokio::test]
async fn fetch_body_caches_the_body_on_the_story_and_fires_event() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(7, StoryState::Ready)]);
    adapter.seed_body(7, "the body text".into());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(store.story(7).unwrap().body, None);

    let mut rx = store.subscribe();
    let returned = orchestrator.fetch_body(7).await.unwrap();
    assert_eq!(returned, "the body text");
    assert_eq!(
        store.story(7).unwrap().body.as_deref(),
        Some("the body text")
    );
    let event = rx.recv().await.unwrap();
    assert!(matches!(event, BoardEvent::BodyFetched { number: 7, .. }));
}

#[tokio::test]
async fn fetch_body_records_error_when_source_fails() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(7, StoryState::Ready)]);
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    let err = orchestrator.fetch_body(7).await.unwrap_err();
    assert!(matches!(err, crate::board::SyncError::SourceUnreachable(_)));
    let status = orchestrator.snapshot_status();
    assert!(status.last_error.unwrap().contains("fetch_body"));
}

#[tokio::test]
async fn fetch_free_comments_caches_comments_and_fires_event() {
    use crate::board::domain::FreeComment;
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(9, StoryState::Ready)]);
    let seeded = vec![FreeComment {
        body: "hi".into(),
        author: "ryan".into(),
        ts: Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap(),
    }];
    adapter.seed_free_comments(9, seeded.clone());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(store.story(9).unwrap().free_comments, None);

    let mut rx = store.subscribe();
    let returned = orchestrator.fetch_free_comments(9).await.unwrap();
    assert_eq!(returned, seeded);
    assert_eq!(store.story(9).unwrap().free_comments.unwrap(), seeded);
    let event = rx.recv().await.unwrap();
    assert!(matches!(
        event,
        BoardEvent::FreeCommentsFetched { number: 9, .. }
    ));
}

#[tokio::test]
async fn fetch_gate_comments_caches_comments_and_fires_event() {
    use crate::board::domain::{Gate, GateComment};
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(11, StoryState::Ready)]);
    let seeded = vec![GateComment {
        gate: Gate::Pickup,
        ts: Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap(),
        actor: "dev".into(),
        payload: Some("paraphrase".into()),
    }];
    adapter.seed_gate_comments(11, seeded.clone());
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(store.story(11).unwrap().gate_comments, None);

    let mut rx = store.subscribe();
    let returned = orchestrator.fetch_gate_comments(11).await.unwrap();
    assert_eq!(returned, seeded);
    assert_eq!(store.story(11).unwrap().gate_comments.unwrap(), seeded);
    let event = rx.recv().await.unwrap();
    assert!(matches!(
        event,
        BoardEvent::GateCommentsFetched { number: 11, .. }
    ));
}

#[tokio::test]
async fn fetch_gate_comments_records_error_when_source_fails() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(11, StoryState::Ready)]);
    adapter.set_ingest_failures(true);
    let orchestrator = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
    let err = orchestrator.fetch_gate_comments(11).await.unwrap_err();
    assert!(matches!(err, crate::board::SyncError::SourceUnreachable(_)));
    assert!(
        orchestrator
            .snapshot_status()
            .last_error
            .unwrap()
            .contains("fetch_gate_comments")
    );
}

#[tokio::test]
async fn drain_pending_via_refresh_driver_is_equivalent_to_drain_once() {
    use crate::board::RefreshDriver;
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(1, StoryState::Ready)]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();
    orchestrator
        .mutate_sync(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 1,
            new_state: StoryState::Delivered,
        })
        .unwrap();
    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    assert_eq!(adapter.pushed_events().len(), 1);
}

#[tokio::test]
async fn reconcile_dual_state_labels_emits_tag_remove_for_stale_in_progress_label() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut delivered = seed_story(450, StoryState::Delivered);
    delivered.story_type = StoryType::Bug;
    adapter.seed_stories(vec![delivered]);
    adapter.seed_labels(
        450,
        vec![
            "in progress".to_string(),
            "delivered".to_string(),
            "bug".to_string(),
        ],
    );
    let orch = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.ingest_once().await.unwrap();
    let repairs = orch.reconcile_dual_state_labels().await.unwrap();
    assert_eq!(repairs, 1, "stale 'in progress' label should fire 1 repair");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagRemoved { number: 450, label, .. } if label == "in progress"
    ));
}

#[tokio::test]
async fn reconcile_dual_state_labels_is_no_op_when_labels_match_state() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(451, StoryState::Delivered)]);
    adapter.seed_labels(451, vec!["delivered".to_string(), "feature".to_string()]);
    let orch = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.ingest_once().await.unwrap();
    let repairs = orch.reconcile_dual_state_labels().await.unwrap();
    assert_eq!(repairs, 0, "clean labels should fire 0 repairs");
    assert_eq!(adapter.pushed_events().len(), 0);
}

mod staleness {
    use super::*;
    use crate::board::orchestrator::is_stale;

    #[test]
    fn no_ingest_yet_is_not_stale() {
        let now = Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap();
        assert!(
            !is_stale(None, 180, now),
            "fresh session is connecting, not stale"
        );
    }

    #[test]
    fn recent_ingest_is_not_stale() {
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        let now = last + chrono::Duration::seconds(60);
        assert!(
            !is_stale(Some(last), 180, now),
            "60s since last ingest under 180s threshold = fresh"
        );
    }

    #[test]
    fn ingest_older_than_threshold_is_stale() {
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        let now = last + chrono::Duration::seconds(181);
        assert!(
            is_stale(Some(last), 180, now),
            "181s since last ingest exceeds 180s threshold = stale"
        );
    }

    #[test]
    fn ingest_at_exact_threshold_is_not_stale() {
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        let now = last + chrono::Duration::seconds(180);
        assert!(
            !is_stale(Some(last), 180, now),
            "exactly threshold seconds is the boundary, not yet stale"
        );
    }

    #[test]
    fn future_ingest_timestamp_is_not_stale() {
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        let now = last - chrono::Duration::seconds(10);
        assert!(
            !is_stale(Some(last), 180, now),
            "clock skew (last in future) doesn't trip stale"
        );
    }

    #[tokio::test]
    async fn snapshot_status_default_threshold_keeps_fresh_ingest_not_stale() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        orch.ingest_once().await.unwrap();
        let snap = orch.snapshot_status();
        assert!(snap.last_ingest_at.is_some(), "ingest happened");
        assert!(
            !snap.stale,
            "ingest milliseconds ago is well under default 180s threshold: {snap:?}"
        );
        assert!(!snap.startup_fail, "successful ingest is not startup-fail");
    }

    #[test]
    fn startup_fail_when_no_ingest_yet_and_last_error_set() {
        use crate::board::orchestrator::is_startup_fail;
        assert!(is_startup_fail(None, Some("rate-limited")));
    }

    #[test]
    fn no_startup_fail_when_ingest_succeeded_even_with_recent_error() {
        use crate::board::orchestrator::is_startup_fail;
        let last = Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap();
        assert!(!is_startup_fail(
            Some(last),
            Some("post-success transient error")
        ));
    }

    #[test]
    fn no_startup_fail_when_no_ingest_yet_but_no_error_recorded() {
        use crate::board::orchestrator::is_startup_fail;
        assert!(
            !is_startup_fail(None, None),
            "fresh-session-pre-first-tick is not startup-fail until an error has been recorded"
        );
    }

    #[tokio::test]
    async fn snapshot_status_surfaces_startup_fail_when_ingest_fails_with_no_prior_success() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.set_ingest_failures(true);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        let _ = orch.ingest_once().await;
        let snap = orch.snapshot_status();
        assert!(snap.last_ingest_at.is_none(), "no successful ingest yet");
        assert!(snap.last_error.is_some(), "ingest error recorded");
        assert!(
            snap.startup_fail,
            "no-ingest + last_error → startup_fail surfaces in snapshot: {snap:?}"
        );
    }
}

mod outbox_persistence {
    use super::*;

    fn outbox_path(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("board-outbox.json")
    }

    #[tokio::test]
    async fn with_outbox_path_starts_empty_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch =
            SyncOrchestrator::with_outbox_path(Arc::clone(&store), Arc::clone(&adapter), path)
                .unwrap();
        assert_eq!(orch.snapshot_status().pending_events, 0);
    }

    #[tokio::test]
    async fn mutate_persists_event_to_disk_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(10, StoryState::Ready)]);
        let orch = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store),
            Arc::clone(&adapter),
            path.clone(),
        )
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 10,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        assert!(path.exists(), "outbox file must exist after mutate");
        // #1195: outbox-on-disk schema is now Vec<QueuedEvent>
        // (event + pre_state wrapper). Use the public load helper
        // so the assertion stays decoupled from the internal shape.
        let events = super::super::orchestrator::load_outbox_from_disk(&path).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn drain_once_removes_pushed_event_from_disk_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(20, StoryState::Ready)]);
        let orch = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store),
            Arc::clone(&adapter),
            path.clone(),
        )
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 20,
            new_state: StoryState::Delivered,
        })
        .unwrap();
        let drained = orch.drain_once().await.unwrap();
        assert_eq!(drained, 1);
        let events = super::super::orchestrator::load_outbox_from_disk(&path).unwrap();
        assert!(events.is_empty(), "outbox empty after successful drain");
    }

    #[tokio::test]
    async fn drain_once_keeps_event_on_disk_when_push_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(30, StoryState::Ready)]);
        let orch = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store),
            Arc::clone(&adapter),
            path.clone(),
        )
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 30,
            new_state: StoryState::Delivered,
        })
        .unwrap();
        adapter.set_push_failures(true);
        let _ = orch.drain_once().await;
        let events = super::super::orchestrator::load_outbox_from_disk(&path).unwrap();
        assert_eq!(events.len(), 1, "failed event stays in outbox");
    }

    #[tokio::test]
    async fn outbox_survives_restart_simulated_via_fresh_orchestrator() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store_a = Arc::new(BoardStore::new());
        let adapter_a = Arc::new(InMemorySourceAdapter::new());
        store_a.seed_all(vec![seed_story(40, StoryState::Ready)]);
        let orch_a = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store_a),
            Arc::clone(&adapter_a),
            path.clone(),
        )
        .unwrap();
        orch_a
            .mutate(BoardEvent::StateTransitioned {
                meta: meta(),
                number: 40,
                new_state: StoryState::Delivered,
            })
            .unwrap();
        drop(orch_a);
        let store_b = Arc::new(BoardStore::new());
        let adapter_b = Arc::new(InMemorySourceAdapter::new());
        let orch_b = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store_b),
            Arc::clone(&adapter_b),
            path.clone(),
        )
        .unwrap();
        assert_eq!(orch_b.snapshot_status().pending_events, 1);
        let drained = orch_b.drain_once().await.unwrap();
        assert_eq!(drained, 1, "rehydrated event drains on fresh orchestrator");
        assert_eq!(adapter_b.pushed_events().len(), 1);
    }

    #[tokio::test]
    async fn drain_drops_event_on_permanent_push_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(60, StoryState::Ready)]);
        let orch = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store),
            Arc::clone(&adapter),
            path.clone(),
        )
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 60,
            new_state: StoryState::Delivered,
        })
        .unwrap();
        adapter.set_permanent_push_failures(true);
        let _ = orch.drain_once().await;
        let bytes = std::fs::read(&path).unwrap();
        let events: Vec<BoardEvent> = serde_json::from_slice(&bytes).unwrap();
        assert!(
            events.is_empty(),
            "permanent failure drops event from outbox"
        );
        assert_eq!(orch.snapshot_status().pending_events, 0);
    }

    #[tokio::test]
    async fn permanent_failure_records_dropped_event_in_status() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(65, StoryState::Ready)]);
        let orch = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store),
            Arc::clone(&adapter),
            path.clone(),
        )
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 65,
            new_state: StoryState::Delivered,
        })
        .unwrap();
        adapter.set_permanent_push_failures(true);
        let _ = orch.drain_once().await;
        let status = orch.snapshot_status();
        assert_eq!(status.dropped_events.len(), 1);
        assert_eq!(status.dropped_events[0].story_number, 65);
    }

    #[tokio::test]
    async fn acknowledge_dropped_clears_dropped_events() {
        use crate::board::orchestrator::acknowledge_dropped;
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(66, StoryState::Ready)]);
        let orch = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store),
            Arc::clone(&adapter),
            path.clone(),
        )
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 66,
            new_state: StoryState::Delivered,
        })
        .unwrap();
        adapter.set_permanent_push_failures(true);
        let _ = orch.drain_once().await;
        assert_eq!(orch.snapshot_status().dropped_events.len(), 1);
        let cleared = acknowledge_dropped(&orch.status_handle());
        assert_eq!(cleared, 1);
        assert!(orch.snapshot_status().dropped_events.is_empty());
    }

    #[tokio::test]
    async fn drain_continues_past_permanent_failure_to_next_event() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![
            seed_story(70, StoryState::Ready),
            seed_story(71, StoryState::Ready),
        ]);
        let orch = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store),
            Arc::clone(&adapter),
            path.clone(),
        )
        .unwrap();
        for n in [70u64, 71] {
            orch.mutate(BoardEvent::StateTransitioned {
                meta: meta(),
                number: n,
                new_state: StoryState::Delivered,
            })
            .unwrap();
        }
        adapter.set_permanent_push_failures(true);
        let _ = orch.drain_once().await;
        let bytes = std::fs::read(&path).unwrap();
        let events: Vec<BoardEvent> = serde_json::from_slice(&bytes).unwrap();
        assert!(
            events.is_empty(),
            "drain continues past each permanent failure"
        );
    }

    #[tokio::test]
    async fn drain_keeps_event_on_transient_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(80, StoryState::Ready)]);
        let orch = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store),
            Arc::clone(&adapter),
            path.clone(),
        )
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 80,
            new_state: StoryState::Delivered,
        })
        .unwrap();
        adapter.set_push_failures(true);
        let _ = orch.drain_once().await;
        let events = super::super::orchestrator::load_outbox_from_disk(&path).unwrap();
        assert_eq!(events.len(), 1, "transient failure keeps event for retry");
        adapter.set_push_failures(false);
        orch.drain_once().await.unwrap();
        let events = super::super::orchestrator::load_outbox_from_disk(&path).unwrap();
        assert!(events.is_empty(), "event drains on retry post-recovery");
    }

    #[tokio::test]
    async fn outbox_preserves_fifo_order_across_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let store_a = Arc::new(BoardStore::new());
        let adapter_a = Arc::new(InMemorySourceAdapter::new());
        store_a.seed_all(vec![
            seed_story(50, StoryState::Ready),
            seed_story(51, StoryState::Ready),
            seed_story(52, StoryState::Ready),
        ]);
        let orch_a = SyncOrchestrator::with_outbox_path(
            Arc::clone(&store_a),
            Arc::clone(&adapter_a),
            path.clone(),
        )
        .unwrap();
        for n in [50u64, 51, 52] {
            orch_a
                .mutate(BoardEvent::StateTransitioned {
                    meta: meta(),
                    number: n,
                    new_state: StoryState::InProgress,
                })
                .unwrap();
        }
        drop(orch_a);
        let store_b = Arc::new(BoardStore::new());
        let adapter_b = Arc::new(InMemorySourceAdapter::new());
        let orch_b =
            SyncOrchestrator::with_outbox_path(Arc::clone(&store_b), Arc::clone(&adapter_b), path)
                .unwrap();
        orch_b.drain_once().await.unwrap();
        let pushed = adapter_b.pushed_events();
        assert_eq!(pushed[0].story_number(), 50);
        assert_eq!(pushed[1].story_number(), 51);
        assert_eq!(pushed[2].story_number(), 52);
    }

    #[tokio::test]
    async fn mutate_silent_no_op_when_story_not_in_store_pins_reducer_shape() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 999,
            new_state: StoryState::InProgress,
        })
        .unwrap();

        assert!(
            store.story(999).is_none(),
            "reducer silently no-ops when story not in store; #1197 fix is at gate-tool layer"
        );
    }

    #[tokio::test]
    async fn refresh_story_populates_store_so_subsequent_mutate_lands() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![seed_story(777, StoryState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 777,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        assert!(
            store.story(777).is_none(),
            "pre-refresh mutate silently no-ops"
        );

        orch.refresh_story(777).await.unwrap();
        assert!(
            store.story(777).is_some(),
            "refresh_story populates the store from adapter"
        );

        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 777,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        assert_eq!(
            store.story(777).map(|s| s.state),
            Some(StoryState::InProgress),
            "post-refresh mutate lands"
        );
    }
}

mod slice_7_2_mutation_id_dispatch {
    use super::*;
    use crate::board::MutationId;
    use crate::orchestrator::OrchestratorAction;

    #[tokio::test]
    async fn mutate_dispatches_start_update_github_with_unique_id() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![
            seed_story(101, StoryState::Ready),
            seed_story(102, StoryState::Ready),
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<OrchestratorAction>();
        let orch =
            SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter)).with_action_tx(tx);
        orch.ingest_once().await.unwrap();

        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 101,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 102,
            new_state: StoryState::InProgress,
        })
        .unwrap();

        let action_a = rx.try_recv().expect("first dispatch");
        let action_b = rx.try_recv().expect("second dispatch");
        let id_a = match action_a {
            OrchestratorAction::StartUpdateGithub { mutation_id } => mutation_id,
            other => panic!("expected StartUpdateGithub, got {other:?}"),
        };
        let id_b = match action_b {
            OrchestratorAction::StartUpdateGithub { mutation_id } => mutation_id,
            other => panic!("expected StartUpdateGithub, got {other:?}"),
        };
        assert_ne!(id_a, id_b, "consecutive mutates allocate distinct ids");
        assert!(id_a.0 >= 1 && id_b.0 >= 1, "ids start at 1, not 0");
    }

    #[tokio::test]
    async fn mutate_without_action_tx_does_not_panic() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![seed_story(201, StoryState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        orch.ingest_once().await.unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 201,
            new_state: StoryState::InProgress,
        })
        .unwrap();
    }

    #[tokio::test]
    async fn queued_event_carries_allocated_id() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![seed_story(301, StoryState::Ready)]);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<OrchestratorAction>();
        let orch =
            SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter)).with_action_tx(tx);
        orch.ingest_once().await.unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 301,
            new_state: StoryState::InProgress,
        })
        .unwrap();

        let action = rx.try_recv().expect("dispatch fires");
        let dispatched_id = match action {
            OrchestratorAction::StartUpdateGithub { mutation_id } => mutation_id,
            other => panic!("expected StartUpdateGithub, got {other:?}"),
        };
        let pending = orch.snapshot_pending_for_test();
        let queued_id = pending.front().map(|q| q.id).expect("one queued event");
        assert_eq!(
            dispatched_id, queued_id,
            "dispatched id matches QueuedEvent.id"
        );
        assert!(
            dispatched_id != MutationId::default(),
            "allocated id is non-default"
        );
    }
}

mod slice_7_3_drain_one_by_id {
    use super::*;
    use crate::board::DrainOneOutcome;
    use crate::board::MutationId;

    #[tokio::test]
    async fn drain_one_by_id_pushes_clean_event_returns_confirmed() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(401, StoryState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 401,
            new_state: StoryState::InProgress,
        })
        .unwrap();

        let id = orch
            .snapshot_pending_for_test()
            .front()
            .map(|q| q.id)
            .expect("queued event has id");
        let outcome = orch.drain_one_by_id(id).await;
        assert!(matches!(outcome, DrainOneOutcome::Confirmed));
        assert_eq!(adapter.pushed_events().len(), 1);
        assert!(orch.snapshot_pending_for_test().is_empty());
    }

    #[tokio::test]
    async fn drain_one_by_id_transient_error_leaves_entry_in_outbox() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(601, StoryState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 601,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        adapter.set_push_failures(true);

        let id = orch
            .snapshot_pending_for_test()
            .front()
            .map(|q| q.id)
            .expect("queued event has id");
        let outcome = orch.drain_one_by_id(id).await;
        assert!(matches!(outcome, DrainOneOutcome::TransientError(_)));
        assert_eq!(orch.snapshot_pending_for_test().len(), 1);
    }

    #[tokio::test]
    async fn drain_one_by_id_returns_not_found_for_unknown_id() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        let outcome = orch.drain_one_by_id(MutationId(99999)).await;
        assert!(matches!(outcome, DrainOneOutcome::NotFound));
    }

    #[tokio::test]
    async fn apply_server_rollback_overwrites_local_optimistic_state() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(701, StoryState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 701,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        assert_eq!(
            store.story(701).map(|s| s.state),
            Some(StoryState::InProgress)
        );

        let server_view = seed_story(701, StoryState::Accepted);
        orch.apply_server_rollback(server_view);
        assert_eq!(
            store.story(701).map(|s| s.state),
            Some(StoryState::Accepted)
        );
    }
}

mod slice_7_4_drain_one_by_id_falsifiers {
    use super::*;
    use crate::board::DrainOneOutcome;

    #[tokio::test]
    async fn permanent_push_failure_drops_entry_returns_confirmed() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(801, StoryState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 801,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        adapter.set_permanent_push_failures(true);

        let id = orch
            .snapshot_pending_for_test()
            .front()
            .map(|q| q.id)
            .expect("queued event has id");
        let outcome = orch.drain_one_by_id(id).await;
        assert!(matches!(outcome, DrainOneOutcome::Confirmed));
        assert!(orch.snapshot_pending_for_test().is_empty());
        assert_eq!(orch.snapshot_status().dropped_events.len(), 1);
    }

    #[tokio::test]
    async fn concurrent_calls_for_different_ids_complete_independently() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![
            seed_story(901, StoryState::Ready),
            seed_story(902, StoryState::Ready),
        ]);
        let orch = Arc::new(SyncOrchestrator::new(
            Arc::clone(&store),
            Arc::clone(&adapter),
        ));
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 901,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 902,
            new_state: StoryState::InProgress,
        })
        .unwrap();

        let pending = orch.snapshot_pending_for_test();
        let id_a = pending[0].id;
        let id_b = pending[1].id;
        let orch_a = Arc::clone(&orch);
        let orch_b = Arc::clone(&orch);
        let (out_a, out_b) = tokio::join!(
            async move { orch_a.drain_one_by_id(id_a).await },
            async move { orch_b.drain_one_by_id(id_b).await },
        );
        assert!(matches!(out_a, DrainOneOutcome::Confirmed));
        assert!(matches!(out_b, DrainOneOutcome::Confirmed));
        assert!(orch.snapshot_pending_for_test().is_empty());
        assert_eq!(adapter.pushed_events().len(), 2);
    }

    #[tokio::test]
    async fn serial_calls_drain_in_request_order_for_same_issue() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(1001, StoryState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 1001,
            new_state: StoryState::InProgress,
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 1001,
            label: "blocked".to_string(),
        })
        .unwrap();

        let pending = orch.snapshot_pending_for_test();
        let id_first = pending[0].id;
        let id_second = pending[1].id;

        let outcome_first = orch.drain_one_by_id(id_first).await;
        let outcome_second = orch.drain_one_by_id(id_second).await;
        assert!(matches!(outcome_first, DrainOneOutcome::Confirmed));
        assert!(matches!(outcome_second, DrainOneOutcome::Confirmed));

        let pushed = adapter.pushed_events();
        assert_eq!(pushed.len(), 2);
        assert!(matches!(pushed[0], BoardEvent::StateTransitioned { .. }));
        assert!(matches!(pushed[1], BoardEvent::TagAdded { .. }));
    }
}

mod outbox_coalesce_1339_step_2e {
    use super::*;

    fn extract_label(event: &BoardEvent) -> Option<(&'static str, &str)> {
        match event {
            BoardEvent::TagAdded { label, .. } => Some(("A", label.as_str())),
            BoardEvent::TagRemoved { label, .. } => Some(("R", label.as_str())),
            _ => None,
        }
    }

    fn queue_labels(orch: &SyncOrchestrator<InMemorySourceAdapter>) -> Vec<(String, String)> {
        orch.snapshot_pending_for_test()
            .iter()
            .filter_map(|q| extract_label(&q.event).map(|(k, l)| (k.to_string(), l.to_string())))
            .collect()
    }

    #[tokio::test]
    async fn rapid_drag_sequence_coalesces_to_first_remove_plus_last_add_per_class() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let mut seed = seed_story(7011, StoryState::Icebox);
        seed.raw_labels = vec!["iteration-Pegasus.300".to_string()];
        store.seed_all(vec![seed]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7011,
            label: "iteration-Pegasus.150".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagRemoved {
            meta: meta(),
            number: 7011,
            label: "iteration-Pegasus.300".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7011,
            label: "iteration-Pegasus.75".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagRemoved {
            meta: meta(),
            number: 7011,
            label: "iteration-Pegasus.150".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7011,
            label: "iteration-Pegasus.50".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagRemoved {
            meta: meta(),
            number: 7011,
            label: "iteration-Pegasus.75".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels,
            vec![
                ("R".to_string(), "iteration-Pegasus.300".to_string()),
                ("A".to_string(), "iteration-Pegasus.50".to_string()),
            ],
            "rapid drag should coalesce to first-remove + last-add: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_per_class_keeps_distinct_iteration_namespaces_apart() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(7012, StoryState::Icebox)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7012,
            label: "iteration-Pegasus.100".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7012,
            label: "iteration-order.50".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7012,
            label: "iteration-Pegasus.50".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels,
            vec![
                ("A".to_string(), "iteration-order.50".to_string()),
                ("A".to_string(), "iteration-Pegasus.50".to_string()),
            ],
            "different prefix-classes coalesce independently: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_does_not_collapse_across_stories() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![
            seed_story(7013, StoryState::Icebox),
            seed_story(7014, StoryState::Icebox),
        ]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7013,
            label: "iteration-Pegasus.100".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7014,
            label: "iteration-Pegasus.100".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7013,
            label: "iteration-Pegasus.50".to_string(),
        })
        .unwrap();

        let pending = orch.snapshot_pending_for_test();
        let by_story: Vec<(u64, String)> = pending
            .iter()
            .filter_map(|q| match &q.event {
                BoardEvent::TagAdded { number, label, .. } => Some((*number, label.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            by_story,
            vec![
                (7014, "iteration-Pegasus.100".to_string()),
                (7013, "iteration-Pegasus.50".to_string()),
            ],
            "cross-story events stay independent; only intra-story coalesces: {by_story:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_does_not_touch_non_iteration_labels() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(7015, StoryState::Ready)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7015,
            label: "blocked".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagRemoved {
            meta: meta(),
            number: 7015,
            label: "ready".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7015,
            label: "in progress".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels.len(),
            3,
            "non-iteration labels do not coalesce: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_handles_add_only_sequence_keeping_last_add() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(7016, StoryState::Icebox)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7016,
            label: "iteration-Pegasus.200".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7016,
            label: "iteration-Pegasus.150".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 7016,
            label: "iteration-Pegasus.100".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels,
            vec![("A".to_string(), "iteration-Pegasus.100".to_string())],
            "Add-only coalesces to last Add: {labels:?}"
        );
    }
}

/// #1382: substrate test for SyncOrchestrator::with_codec stub
/// injection. A StubLabelCodec with non-canonical syntax
/// (`STUB|Theme:N`) is injected into the orchestrator; the coalescer
/// must classify the stub's iteration-shape via the injected codec
/// rather than the hardcoded GhLabelCodec. Pre-#1382 this was
/// impossible because iteration_prefix_class inlined GhLabelCodec.
mod codec_injection_substrate {
    use super::*;
    use crate::board::iteration_label::IterationLabelParts;
    use crate::board::label_codec::LabelCodec;
    use std::sync::Arc;

    struct StubLabelCodec;

    impl LabelCodec for StubLabelCodec {
        fn decode_ready_order(&self, label: &str) -> Option<u32> {
            label.strip_prefix("rdy:").and_then(|s| s.parse().ok())
        }

        fn encode_ready_order(&self, position: u32) -> String {
            format!("rdy:{position}")
        }

        fn decode_iteration(&self, label: &str) -> Option<IterationLabelParts> {
            let rest = label.strip_prefix("STUB|")?;
            let (theme, pos) = rest.split_once(':')?;
            let position: u32 = pos.parse().ok()?;
            Some(IterationLabelParts {
                theme_name: theme.to_string(),
                position,
            })
        }

        fn is_iteration_label_loose(&self, label: &str) -> bool {
            label.starts_with("STUB|") && label.contains(':')
        }

        fn encode_iteration(&self, theme_name: &str, position: u32) -> String {
            format!("STUB|{theme_name}:{position}")
        }

        fn iteration_label_class(&self, label: &str) -> Option<String> {
            let rest = label.strip_prefix("STUB|")?;
            let (theme, _pos) = rest.split_once(':')?;
            Some(format!("STUB|{theme}"))
        }

        fn encode_parent_epic(&self, issue_number: u64) -> String {
            format!("epic#{issue_number}")
        }
    }

    fn meta() -> EventMeta {
        EventMeta {
            actor: "stub".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).unwrap(),
            peer_id: PeerId::local(),
        }
    }

    fn seed_story(number: u64) -> Story {
        Story::new_minimal(
            number,
            format!("story-{number}"),
            StoryState::InProgress,
            Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).unwrap(),
            "pm".to_string(),
        )
    }

    fn queue_labels(orch: &SyncOrchestrator<InMemorySourceAdapter>) -> Vec<(String, String)> {
        orch.snapshot_pending_for_test()
            .iter()
            .map(|q| {
                let (k, l) = match &q.event {
                    BoardEvent::TagAdded { label, .. } => ("A", label.clone()),
                    BoardEvent::TagRemoved { label, .. } => ("R", label.clone()),
                    _ => ("?", String::new()),
                };
                (k.to_string(), l)
            })
            .collect()
    }

    #[tokio::test]
    async fn coalescer_uses_injected_codec_classification() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(8801)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter))
            .with_codec(Arc::new(StubLabelCodec));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 8801,
            label: "STUB|Andromeda:100".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 8801,
            label: "STUB|Andromeda:200".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels,
            vec![("A".to_string(), "STUB|Andromeda:200".to_string())],
            "stub codec's iteration_label_class drove coalesce: {labels:?}"
        );
    }

    #[tokio::test]
    async fn coalescer_keeps_distinct_stub_iteration_namespaces_apart() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(8802)]);
        let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter))
            .with_codec(Arc::new(StubLabelCodec));

        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 8802,
            label: "STUB|Andromeda:100".to_string(),
        })
        .unwrap();
        orch.mutate(BoardEvent::TagAdded {
            meta: meta(),
            number: 8802,
            label: "STUB|Pegasus:50".to_string(),
        })
        .unwrap();

        let labels = queue_labels(&orch);
        assert_eq!(
            labels.len(),
            2,
            "distinct iteration-class labels preserved by stub codec: {labels:?}"
        );
    }
}

#[cfg(test)]
mod race_window_barrier {
    //! #1406 substrate-tests for `RaceWindowBarrier` — the test-only
    //! counted leader/follower rendezvous installed via
    //! `with_race_window_barrier` from
    //! `BPMUX_TEST_RACE_WINDOW_REQUESTS=N` on the server subprocess.
    //! Production servers never set the env var so the orchestrator's
    //! `race_window_barrier` field stays None + the trait default
    //! `test_race_window_*` methods are no-ops.
    //!
    //! Leader/follower happens-before semantics are verified in
    //! production via the #1421-blocked e2e — `tokio::sync::Barrier`
    //! designates the LAST arriver as leader (not the first), and
    //! since `arrive()` does not surface is_leader to the caller, a
    //! unit-axis test cannot deterministically dispatch "leader work"
    //! to the right task without exposing the designation.
    //! The basic API contract (target=0 pass-through, complete
    //! idempotence under target=N) IS unit-testable here; the
    //! happens-before contract belongs to the integration-axis.
    //! Production usage is symmetric — both fires of
    //! handle_accept_story call arrive() then the same critical-
    //! section then complete(); Barrier's is_leader assignment is
    //! transparent to the handler code.

    use crate::board::RaceWindowBarrier;
    use std::sync::Arc;

    #[tokio::test]
    async fn target_zero_arrive_is_pass_through() {
        let barrier = RaceWindowBarrier::new(0);
        barrier.arrive().await;
    }

    #[tokio::test]
    async fn target_zero_complete_is_no_op() {
        let barrier = RaceWindowBarrier::new(0);
        barrier.complete();
    }

    #[tokio::test]
    async fn complete_is_idempotent_under_target_two_rendezvous() {
        let barrier = Arc::new(RaceWindowBarrier::new(2));
        let a = Arc::clone(&barrier);
        let b = Arc::clone(&barrier);
        let task_a = tokio::spawn(async move {
            a.arrive().await;
            a.complete();
            a.complete();
        });
        let task_b = tokio::spawn(async move {
            b.arrive().await;
            b.complete();
        });
        task_a.await.expect("task a");
        task_b.await.expect("task b");
    }
}
