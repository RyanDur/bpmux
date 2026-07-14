#![allow(unused_imports)]

use std::path::Path;
use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::test_support::eventually;
use crate::store::{AppAction, SeededStore};
use source_sync::{PendingWrite, QueuedWrite};
use storage::{FileStorage, load_outbox_from_disk};

use crate::sync::{OutboxSink, RefreshDriver, SyncOrchestrator};
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{Stage, Story, StoryId, Work};

mod outbox_persistence {
    use super::*;

    fn outbox_path(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("board-outbox.json")
    }

    fn wire(
        adapter: &Arc<InMemorySourceAdapter>,
        sink: OutboxSink,
        initial: Vec<QueuedWrite>,
        stories: Vec<Story>,
    ) -> (Arc<SyncOrchestrator<InMemorySourceAdapter>>, SeededStore) {
        let orch = Arc::new(SyncOrchestrator::new(
            Arc::clone(adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new_wired_outbox(sink, initial);
        handle.bind_refresh(&orch_dyn);
        if !stories.is_empty() {
            handle.dispatch(AppAction::Story(StoryAction::Seeded(stories)));
        }
        (orch, handle)
    }

    fn state_write(id: u64, new_state: Work) -> StoryEvent {
        StoryEvent::new(
            meta(),
            StoryId::new(id),
            StoryEventKind::StateTransitioned {
                new_state,
                placement: None,
            },
        )
    }

    async fn eventually_disk_len(path: &Path, expected: usize) {
        eventually(
            "the write-behind persister lands the queued snapshot on disk",
            || {
                load_outbox_from_disk(path)
                    .map(|events| events.len())
                    .unwrap_or(usize::MAX)
                    == expected
            },
        )
        .await;
    }

    #[tokio::test]
    async fn hydrating_a_missing_outbox_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let loaded = load_outbox_from_disk(&path).unwrap();
        assert!(loaded.is_empty(), "a missing outbox file hydrates empty");
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path),
            )),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(orch.snapshot_status().pending_events, 0);
    }

    #[tokio::test]
    async fn mutate_persists_event_to_disk_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![seed_story(10, Work::NotStarted)],
        );
        orch.mutate(state_write(10, Work::InProgress)).unwrap();
        eventually_disk_len(&path, 1).await;
        assert!(path.exists(), "outbox file must exist after mutate");
    }

    #[tokio::test]
    async fn drain_pass_removes_pushed_event_from_disk_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![seed_story(20, Work::NotStarted)],
        );
        orch.mutate(state_write(20, Work::Acceptance)).unwrap();
        eventually_disk_len(&path, 1).await;
        let drained = orch.drain_pass().await.unwrap();
        assert_eq!(drained, 1);
        eventually_disk_len(&path, 0).await;
    }

    #[tokio::test]
    async fn drain_drops_pushed_event_from_storage_outbox() {
        use source_sync::OutboxRepo;
        use storage::{SqliteStorage, Storage};
        let dir = tempfile::tempdir().unwrap();
        let storage: Arc<dyn Storage> =
            Arc::new(SqliteStorage::new(dir.path().to_path_buf()).unwrap());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::clone(&storage)),
            Vec::new(),
            vec![seed_story(90, Work::NotStarted)],
        );
        orch.mutate(state_write(90, Work::Acceptance)).unwrap();
        eventually(
            "mutate persists the pending write to the storage outbox",
            || storage.load_outbox().map(|e| e.len()).unwrap_or(usize::MAX) == 1,
        )
        .await;

        let drained = orch.drain_pass().await.unwrap();
        assert_eq!(drained, 1);
        eventually(
            "a successfully-pushed record is dropped from the persisted storage outbox",
            || storage.load_outbox().map(|e| e.is_empty()).unwrap_or(false),
        )
        .await;
    }

    #[tokio::test]
    async fn drain_pass_keeps_event_on_disk_when_push_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![seed_story(30, Work::NotStarted)],
        );
        orch.mutate(state_write(30, Work::Acceptance)).unwrap();
        eventually_disk_len(&path, 1).await;
        adapter.set_push_failures(true);
        let _ = orch.drain_pass().await;
        let events = load_outbox_from_disk(&path).unwrap();
        assert_eq!(events.len(), 1, "failed event stays in outbox");
    }

    #[tokio::test]
    async fn outbox_survives_restart_simulated_via_fresh_orchestrator() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter_a = Arc::new(InMemorySourceAdapter::new());
        let (orch_a, handle_a) = wire(
            &adapter_a,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![seed_story(40, Work::NotStarted)],
        );
        orch_a.mutate(state_write(40, Work::Acceptance)).unwrap();
        eventually_disk_len(&path, 1).await;
        drop(orch_a);
        drop(handle_a);

        let loaded = load_outbox_from_disk(&path).unwrap();
        assert_eq!(
            loaded.len(),
            1,
            "the row survives on disk across the restart"
        );
        let hydrated: Vec<QueuedWrite> = loaded
            .into_iter()
            .map(|row| {
                QueuedWrite::minted(
                    row.id,
                    PendingWrite::from_event(
                        &source_codec::gh_label_codec::GhLabelCodec,
                        row.event,
                    ),
                )
            })
            .collect();

        let adapter_b = Arc::new(InMemorySourceAdapter::new());
        let (orch_b, _handle_b) = wire(
            &adapter_b,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            hydrated,
            Vec::new(),
        );
        assert_eq!(orch_b.snapshot_status().pending_events, 1);
        let drained = orch_b.drain_pass().await.unwrap();
        assert_eq!(drained, 1, "rehydrated event drains on fresh orchestrator");
        assert_eq!(adapter_b.pushed_events().len(), 1);
    }

    #[tokio::test]
    async fn drain_drops_event_on_permanent_push_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![seed_story(60, Work::NotStarted)],
        );
        orch.mutate(state_write(60, Work::Acceptance)).unwrap();
        eventually_disk_len(&path, 1).await;
        adapter.set_permanent_push_failures(true);
        let _ = orch.drain_pass().await;
        eventually_disk_len(&path, 0).await;
        assert_eq!(orch.snapshot_status().pending_events, 0);
    }

    #[tokio::test]
    async fn permanent_failure_records_dropped_event_in_status() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![seed_story(65, Work::NotStarted)],
        );
        orch.mutate(state_write(65, Work::Acceptance)).unwrap();
        eventually_disk_len(&path, 1).await;
        adapter.set_permanent_push_failures(true);
        let _ = orch.drain_pass().await;
        let status = orch.snapshot_status();
        assert_eq!(status.dropped_events.len(), 1);
        assert_eq!(status.dropped_events[0].story_id, 65);
    }

    #[tokio::test]
    async fn acknowledge_dropped_clears_dropped_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![seed_story(66, Work::NotStarted)],
        );
        orch.mutate(state_write(66, Work::Acceptance)).unwrap();
        eventually_disk_len(&path, 1).await;
        adapter.set_permanent_push_failures(true);
        let _ = orch.drain_pass().await;
        assert_eq!(orch.snapshot_status().dropped_events.len(), 1);
        let cleared = orch.acknowledge_dropped();
        assert_eq!(cleared, 1);
        assert!(orch.snapshot_status().dropped_events.is_empty());
    }

    #[tokio::test]
    async fn drain_continues_past_permanent_failure_to_next_event() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![
                seed_story(70, Work::NotStarted),
                seed_story(71, Work::NotStarted),
            ],
        );
        orch.mutate(state_write(70, Work::Acceptance)).unwrap();
        orch.mutate(state_write(71, Work::Acceptance)).unwrap();
        eventually_disk_len(&path, 2).await;
        adapter.set_permanent_push_failures(true);
        let _ = orch.drain_pass().await;
        eventually_disk_len(&path, 0).await;
    }

    #[tokio::test]
    async fn drain_keeps_event_on_transient_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (orch, _handle) = wire(
            &adapter,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![seed_story(80, Work::NotStarted)],
        );
        orch.mutate(state_write(80, Work::Acceptance)).unwrap();
        eventually_disk_len(&path, 1).await;
        adapter.set_push_failures(true);
        let _ = orch.drain_pass().await;
        let events = load_outbox_from_disk(&path).unwrap();
        assert_eq!(events.len(), 1, "transient failure keeps event for retry");
        adapter.set_push_failures(false);
        orch.drain_pass().await.unwrap();
        eventually_disk_len(&path, 0).await;
    }

    #[tokio::test]
    async fn outbox_preserves_fifo_order_across_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = outbox_path(&dir);
        let adapter_a = Arc::new(InMemorySourceAdapter::new());
        let (orch_a, handle_a) = wire(
            &adapter_a,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            Vec::new(),
            vec![
                seed_story(50, Work::NotStarted),
                seed_story(51, Work::NotStarted),
                seed_story(52, Work::NotStarted),
            ],
        );
        for n in [50u64, 51, 52] {
            orch_a.mutate(state_write(n, Work::InProgress)).unwrap();
        }
        eventually_disk_len(&path, 3).await;
        drop(orch_a);
        drop(handle_a);

        let hydrated: Vec<QueuedWrite> = load_outbox_from_disk(&path)
            .unwrap()
            .into_iter()
            .map(|row| {
                QueuedWrite::minted(
                    row.id,
                    PendingWrite::from_event(
                        &source_codec::gh_label_codec::GhLabelCodec,
                        row.event,
                    ),
                )
            })
            .collect();

        let adapter_b = Arc::new(InMemorySourceAdapter::new());
        let (orch_b, _handle_b) = wire(
            &adapter_b,
            OutboxSink::Storage(Arc::new(
                FileStorage::new(dir.path().to_path_buf()).with_outbox_path(path.clone()),
            )),
            hydrated,
            Vec::new(),
        );
        orch_b.drain_pass().await.unwrap();
        let pushed = adapter_b.pushed_events();
        assert_eq!(pushed[0].story_id(), StoryId::new(50));
        assert_eq!(pushed[1].story_id(), StoryId::new(51));
        assert_eq!(pushed[2].story_id(), StoryId::new(52));
    }

    #[tokio::test]
    async fn mutate_silent_no_op_when_story_not_in_store_pins_reducer_shape() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let orch = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);

        orch.mutate(state_write(999, Work::InProgress)).unwrap();

        assert!(
            orch.story(999).is_none(),
            "reducer silently no-ops when story not in store; #1197 fix is at gate-tool layer"
        );
    }

    #[tokio::test]
    async fn refresh_story_populates_store_so_subsequent_mutate_lands() {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![seed_story(777, Work::NotStarted)]);
        let orch = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
        let handle = SeededStore::new(vec![], vec![], vec![]);
        handle.bind_refresh(&orch_dyn);

        orch.mutate(state_write(777, Work::InProgress)).unwrap();
        assert!(
            orch.story(777).is_none(),
            "pre-refresh mutate silently no-ops"
        );

        orch.refresh_story(777).await.unwrap();
        assert!(
            orch.story(777).is_some(),
            "refresh_story populates the store from adapter"
        );

        orch.mutate(state_write(777, Work::InProgress)).unwrap();
        assert_eq!(
            orch.story(777).map(|s| s.state),
            Some(Work::InProgress),
            "post-refresh mutate lands"
        );
    }
}
