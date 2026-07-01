#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::events::{BoardEvent, EventMeta};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{BoardState, Story, StoryId};

#[tokio::test]
async fn drain_pending_via_refresh_driver_is_equivalent_to_drain_once() {
    use crate::product_management::board::RefreshDriver;
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(1, BoardState::Ready)]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();
    orchestrator
        .mutate_sync(BoardEvent::StateTransitioned {
            meta: meta(),
            id: StoryId::new(1),
            new_state: BoardState::Delivered,
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
    let mut delivered = seed_story(450, BoardState::Delivered);
    delivered.raw_labels.push("bug".to_string());
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
        BoardEvent::TagRemoved { id, label, .. } if *id == StoryId::new(450) && label == "in progress"
    ));
}

#[tokio::test]
async fn reconcile_dual_state_labels_is_no_op_when_labels_match_state() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(451, BoardState::Delivered)]);
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
