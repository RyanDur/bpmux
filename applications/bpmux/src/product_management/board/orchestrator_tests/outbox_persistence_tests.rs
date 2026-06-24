#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::domain::{Story, StoryState, StoryType};
use crate::product_management::board::events::{BoardEvent, EventMeta};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::store::BoardStore;
use crate::storage::{SqliteStorage, Storage};

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
        let events =
            crate::product_management::board::orchestrator::load_outbox_from_disk(&path).unwrap();
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
        let events =
            crate::product_management::board::orchestrator::load_outbox_from_disk(&path).unwrap();
        assert!(events.is_empty(), "outbox empty after successful drain");
    }

    #[tokio::test]
    async fn drain_drops_pushed_event_from_storage_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let storage: Arc<dyn Storage> =
            Arc::new(SqliteStorage::new(dir.path().to_path_buf()).unwrap());
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        store.seed_all(vec![seed_story(90, StoryState::Ready)]);
        let orch = SyncOrchestrator::with_storage(
            Arc::clone(&store),
            Arc::clone(&adapter),
            Arc::clone(&storage),
        )
        .unwrap();
        orch.mutate(BoardEvent::StateTransitioned {
            meta: meta(),
            number: 90,
            new_state: StoryState::Delivered,
        })
        .unwrap();
        assert_eq!(
            storage.load_outbox().unwrap().len(),
            1,
            "mutate persists the pending write to the storage outbox"
        );

        let drained = orch.drain_once().await.unwrap();
        assert_eq!(drained, 1);
        assert!(
            storage.load_outbox().unwrap().is_empty(),
            "a successfully-pushed record is dropped from the persisted storage outbox"
        );
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
        let events =
            crate::product_management::board::orchestrator::load_outbox_from_disk(&path).unwrap();
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
        use crate::product_management::board::orchestrator::acknowledge_dropped;
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
        let events =
            crate::product_management::board::orchestrator::load_outbox_from_disk(&path).unwrap();
        assert_eq!(events.len(), 1, "transient failure keeps event for retry");
        adapter.set_push_failures(false);
        orch.drain_once().await.unwrap();
        let events =
            crate::product_management::board::orchestrator::load_outbox_from_disk(&path).unwrap();
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
