#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use integrations::in_memory::InMemorySourceAdapter;
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{Stage, Story, StoryId, Work};
use store::AppAction;
use store_test_support::{SeededStore, build_seeded_orch};
use sync::SyncOrchestrator;

mod mutation_id_dispatch {
    use super::*;
    use source_sync::MutationId;

    #[tokio::test]
    async fn mutate_enqueues_writes_with_unique_ids() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let handle = SeededStore::new(vec![], vec![], vec![]);
        let orch = handle.build_orch(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        );
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![
            seed_story(101, Work::Ready),
            seed_story(102, Work::Ready),
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

        let writes = handle.pending_writes();
        assert_eq!(writes.len(), 2, "each mutate enqueues one write");
        let id_a = writes[0].id;
        let id_b = writes[1].id;
        assert_ne!(id_a, id_b, "consecutive mutates allocate distinct ids");
        assert!(id_a.0 >= 1 && id_b.0 >= 1, "ids start at 1, not 0");
    }

    #[tokio::test]
    async fn mutate_without_board_dispatch_bound_does_not_panic() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![seed_story(201, Work::Ready)]);
        let (orch, _select) = build_seeded_orch(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
            vec![],
            vec![],
            vec![],
        );
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
    async fn queued_write_carries_allocated_id() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let handle = SeededStore::new(vec![], vec![], vec![]);
        let orch = handle.build_orch(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        );
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            301,
            Work::Ready,
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

        let writes = handle.pending_writes();
        let queued_id = writes.first().map(|q| q.id).expect("one queued write");
        assert!(
            queued_id != MutationId::default(),
            "allocated id is non-default"
        );
    }
}

mod drain_pass {
    use super::*;

    #[tokio::test]
    async fn drain_pass_pushes_clean_event() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let handle = SeededStore::new(vec![], vec![], vec![]);
        let orch = handle.build_orch(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        );
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            401,
            Work::Ready,
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
        assert!(handle.pending_writes().is_empty());
    }

    #[tokio::test]
    async fn drain_pass_transient_error_leaves_entry_in_outbox() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let handle = SeededStore::new(vec![], vec![], vec![]);
        let orch = handle.build_orch(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        );
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            601,
            Work::Ready,
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
        assert_eq!(handle.pending_writes().len(), 1);
    }

    #[tokio::test]
    async fn drain_pass_over_empty_queue_is_a_noop() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let handle = SeededStore::new(vec![], vec![], vec![]);
        let orch = handle.build_orch(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        );
        let drained = orch.drain_pass().await.unwrap();
        assert_eq!(drained, 0);
    }
}

mod drain_pass_edge_cases {
    use super::*;

    #[tokio::test]
    async fn permanent_push_failure_drops_entry() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let handle = SeededStore::new(vec![], vec![], vec![]);
        let orch = handle.build_orch(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        );
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            801,
            Work::Ready,
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
        assert!(handle.pending_writes().is_empty());
        assert_eq!(orch.snapshot_status().dropped_events.len(), 1);
    }

    #[tokio::test]
    async fn single_flight_drain_pass_pushes_every_queued_write_once() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let handle = SeededStore::new(vec![], vec![], vec![]);
        let orch = handle.build_orch(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        );
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![
            seed_story(901, Work::Ready),
            seed_story(902, Work::Ready),
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
        assert!(handle.pending_writes().is_empty());
        assert_eq!(adapter.pushed_events().len(), 2);
    }

    #[tokio::test]
    async fn drain_pass_drains_in_request_order_for_same_issue() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let handle = SeededStore::new(vec![], vec![], vec![]);
        let orch = handle.build_orch(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        );
        handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
            1001,
            Work::Ready,
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
