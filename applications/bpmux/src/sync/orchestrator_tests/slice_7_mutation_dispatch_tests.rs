#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Stage, Story, StoryId, Work};
use crate::store::{AppAction, SeededStore, bind_seeded_refresh};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;

mod slice_7_2_mutation_id_dispatch {
    use super::*;
    use crate::store::{AppAction, SyncAction};
    use crate::sync::MutationId;

    #[tokio::test]
    async fn mutate_dispatches_start_update_github_with_unique_id() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
        let orch = Arc::new(SyncOrchestrator::new(adapter).with_action_tx(tx));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![
            seed_story(101, Work::NotStarted),
            seed_story(102, Work::NotStarted),
        ])));

        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(101),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(102),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();

        let action_a = rx.try_recv().expect("first dispatch");
        let action_b = rx.try_recv().expect("second dispatch");
        let id_a = match action_a {
            AppAction::Sync(SyncAction::StartUpdateGithub { mutation_id }) => mutation_id,
            _ => panic!("expected AppAction::Sync(StartUpdateGithub)"),
        };
        let id_b = match action_b {
            AppAction::Sync(SyncAction::StartUpdateGithub { mutation_id }) => mutation_id,
            _ => panic!("expected AppAction::Sync(StartUpdateGithub)"),
        };
        assert_ne!(id_a, id_b, "consecutive mutates allocate distinct ids");
        assert!(id_a.0 >= 1 && id_b.0 >= 1, "ids start at 1, not 0");
    }

    #[tokio::test]
    async fn mutate_without_action_tx_does_not_panic() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![seed_story(201, Work::NotStarted)]);
        let orch = Arc::new(SyncOrchestrator::new(adapter));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        bind_seeded_refresh(&orch_dyn, vec![], vec![], vec![]);
        orch.ingest_once().await.unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(201),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
    }

    #[tokio::test]
    async fn queued_event_carries_allocated_id() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
        let orch = Arc::new(SyncOrchestrator::new(adapter).with_action_tx(tx));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            301,
            Work::NotStarted,
        )])));
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(301),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();

        let action = rx.try_recv().expect("dispatch fires");
        let dispatched_id = match action {
            AppAction::Sync(SyncAction::StartUpdateGithub { mutation_id }) => mutation_id,
            _ => panic!("expected AppAction::Sync(StartUpdateGithub)"),
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

mod slice_7_3_drain_pass {
    use super::*;

    #[tokio::test]
    async fn drain_pass_pushes_clean_event() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            401,
            Work::NotStarted,
        )])));
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(401),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();

        let drained = orch.drain_pass().await.unwrap();
        assert_eq!(drained, 1);
        assert_eq!(adapter.pushed_events().len(), 1);
        assert!(orch.snapshot_pending_for_test().is_empty());
    }

    #[tokio::test]
    async fn drain_pass_transient_error_leaves_entry_in_outbox() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            601,
            Work::NotStarted,
        )])));
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(601),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
        adapter.set_push_failures(true);

        let result = orch.drain_pass().await;
        assert!(result.is_err());
        assert_eq!(orch.snapshot_pending_for_test().len(), 1);
    }

    #[tokio::test]
    async fn drain_pass_over_empty_queue_is_a_noop() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = SyncOrchestrator::new(adapter);
        let drained = orch.drain_pass().await.unwrap();
        assert_eq!(drained, 0);
    }
}

mod slice_7_4_drain_pass_falsifiers {
    use super::*;

    #[tokio::test]
    async fn permanent_push_failure_drops_entry() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            801,
            Work::NotStarted,
        )])));
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(801),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
        adapter.set_permanent_push_failures(true);

        let result = orch.drain_pass().await;
        assert!(result.is_err());
        assert!(orch.snapshot_pending_for_test().is_empty());
        assert_eq!(orch.snapshot_status().dropped_events.len(), 1);
    }

    #[tokio::test]
    async fn single_flight_drain_pass_pushes_every_queued_write_once() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![
            seed_story(901, Work::NotStarted),
            seed_story(902, Work::NotStarted),
        ])));
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(901),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(902),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();

        let orch_a = Arc::clone(&orch);
        let orch_b = Arc::clone(&orch);
        let (out_a, out_b) = tokio::join!(async move { orch_a.drain_pass().await }, async move {
            orch_b.drain_pass().await
        },);
        assert!(out_a.is_ok());
        assert!(out_b.is_ok());
        assert!(orch.snapshot_pending_for_test().is_empty());
        assert_eq!(adapter.pushed_events().len(), 2);
    }

    #[tokio::test]
    async fn drain_pass_drains_in_request_order_for_same_issue() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            1001,
            Work::NotStarted,
        )])));
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(1001),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
        orch.mutate(StoryEvent::new(
            meta(),
            StoryId::new(1001),
            StoryEventKind::TagAdded {
                label: "blocked".to_string(),
            },
        ))
        .unwrap();

        orch.drain_pass().await.unwrap();

        let pushed = adapter.pushed_events();
        assert_eq!(pushed.len(), 2);
        assert!(matches!(
            pushed[0].kind,
            StoryEventKind::StateTransitioned { .. }
        ));
        assert!(matches!(pushed[1].kind, StoryEventKind::TagAdded { .. }));
    }
}
