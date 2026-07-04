#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::events::{EventMeta, StoryEvent};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{BoardState, Stage, Story, StoryId};
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
            seed_story(101, BoardState::Ready),
            seed_story(102, BoardState::Ready),
        ])));

        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(101),
            new_state: BoardState::InProgress(Stage::Dev.id()),
        })
        .unwrap();
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(102),
            new_state: BoardState::InProgress(Stage::Dev.id()),
        })
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
        adapter.seed_stories(vec![seed_story(201, BoardState::Ready)]);
        let orch = Arc::new(SyncOrchestrator::new(adapter));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        bind_seeded_refresh(&orch_dyn, vec![], vec![], vec![]);
        orch.ingest_once().await.unwrap();
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(201),
            new_state: BoardState::InProgress(Stage::Dev.id()),
        })
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
            BoardState::Ready,
        )])));
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(301),
            new_state: BoardState::InProgress(Stage::Dev.id()),
        })
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

mod slice_7_3_drain_one_by_id {
    use super::*;
    use crate::sync::DrainOneOutcome;
    use crate::sync::MutationId;

    #[tokio::test]
    async fn drain_one_by_id_pushes_clean_event_returns_confirmed() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            401,
            BoardState::Ready,
        )])));
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(401),
            new_state: BoardState::InProgress(Stage::Dev.id()),
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
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            601,
            BoardState::Ready,
        )])));
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(601),
            new_state: BoardState::InProgress(Stage::Dev.id()),
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
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = SyncOrchestrator::new(adapter);
        let outcome = orch.drain_one_by_id(MutationId(99999)).await;
        assert!(matches!(outcome, DrainOneOutcome::NotFound));
    }

    #[tokio::test]
    async fn apply_server_rollback_overwrites_local_optimistic_state() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(adapter));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            701,
            BoardState::Ready,
        )])));
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(701),
            new_state: BoardState::InProgress(Stage::Dev.id()),
        })
        .unwrap();
        assert_eq!(
            orch.story(701).map(|s| s.state),
            Some(BoardState::InProgress(Stage::Dev.id()))
        );

        let server_view = seed_story(701, BoardState::Accepted);
        orch.apply_server_rollback(server_view);
        assert_eq!(orch.story(701).map(|s| s.state), Some(BoardState::Accepted));
    }
}

mod slice_7_4_drain_one_by_id_falsifiers {
    use super::*;
    use crate::sync::DrainOneOutcome;

    #[tokio::test]
    async fn permanent_push_failure_drops_entry_returns_confirmed() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            801,
            BoardState::Ready,
        )])));
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(801),
            new_state: BoardState::InProgress(Stage::Dev.id()),
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
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![
            seed_story(901, BoardState::Ready),
            seed_story(902, BoardState::Ready),
        ])));
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(901),
            new_state: BoardState::InProgress(Stage::Dev.id()),
        })
        .unwrap();
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(902),
            new_state: BoardState::InProgress(Stage::Dev.id()),
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
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            1001,
            BoardState::Ready,
        )])));
        orch.mutate(StoryEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(1001),
            new_state: BoardState::InProgress(Stage::Dev.id()),
        })
        .unwrap();
        orch.mutate(StoryEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(1001),
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
        assert!(matches!(pushed[0], StoryEvent::StateTransitioned { .. }));
        assert!(matches!(pushed[1], StoryEvent::TagAdded { .. }));
    }
}
