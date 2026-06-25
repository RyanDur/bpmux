#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::events::{BoardEvent, EventMeta};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{Story, StoryState, StoryType};

mod slice_7_2_mutation_id_dispatch {
    use super::*;
    use crate::product_management::board::MutationId;
    use crate::store::StoreAction;

    #[tokio::test]
    async fn mutate_dispatches_start_update_github_with_unique_id() {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![
            seed_story(101, StoryState::Ready),
            seed_story(102, StoryState::Ready),
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StoreAction>();
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
            StoreAction::StartUpdateGithub { mutation_id } => mutation_id,
            other => panic!("expected StartUpdateGithub, got {other:?}"),
        };
        let id_b = match action_b {
            StoreAction::StartUpdateGithub { mutation_id } => mutation_id,
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
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StoreAction>();
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
            StoreAction::StartUpdateGithub { mutation_id } => mutation_id,
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
    use crate::product_management::board::DrainOneOutcome;
    use crate::product_management::board::MutationId;

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
    use crate::product_management::board::DrainOneOutcome;

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
