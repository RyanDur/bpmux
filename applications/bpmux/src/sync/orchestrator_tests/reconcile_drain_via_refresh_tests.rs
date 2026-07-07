#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{Story, StoryId, Work};
use crate::store::bind_seeded_refresh;
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;

fn current_iteration_of(members: &[u64]) -> Vec<Iteration> {
    vec![Iteration {
        id: IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: members.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }]
}

#[tokio::test]
async fn drain_pending_via_refresh_driver_is_equivalent_to_drain_once() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(1, Work::NotStarted)]);
    let orchestrator: Arc<dyn RefreshDriver> =
        Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();
    orchestrator
        .mutate_sync(StoryEvent::new(
            meta(),
            StoryId::new(1),
            StoryEventKind::StateTransitioned {
                new_state: Work::Acceptance,
                placement: None,
            },
        ))
        .unwrap();
    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    assert_eq!(adapter.pushed_events().len(), 1);
}

#[tokio::test]
async fn reconcile_dual_state_labels_emits_tag_remove_for_stale_in_progress_label() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let delivered = seed_story(450, Work::Acceptance);
    adapter.seed_stories(vec![delivered]);
    adapter.seed_labels(
        450,
        vec![
            "in progress".to_string(),
            "delivered".to_string(),
            "bug".to_string(),
        ],
    );
    let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
    bind_seeded_refresh(&orch_dyn, vec![], vec![], vec![]);
    orch.ingest_once().await.unwrap();
    let repairs = orch.reconcile_dual_state_labels().await.unwrap();
    assert_eq!(repairs, 1, "stale 'in progress' label should fire 1 repair");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::TagRemoved { label } if pushed[0].id == StoryId::new(450) && label == "in progress"
    ));
}

#[tokio::test]
async fn reconcile_dual_state_labels_is_no_op_when_labels_match_state() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(451, Work::Acceptance)]);
    adapter.seed_labels(451, vec!["delivered".to_string(), "feature".to_string()]);
    let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
    bind_seeded_refresh(&orch_dyn, vec![], vec![], vec![]);
    orch.ingest_once().await.unwrap();
    let repairs = orch.reconcile_dual_state_labels().await.unwrap();
    assert_eq!(repairs, 0, "clean labels should fire 0 repairs");
    assert_eq!(adapter.pushed_events().len(), 0);
}

#[tokio::test]
async fn reconcile_keeps_the_ready_label_for_a_current_iteration_member() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(452, Work::NotStarted)]);
    adapter.seed_labels(452, vec!["ready".to_string(), "feature".to_string()]);
    let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
    bind_seeded_refresh(&orch_dyn, vec![], current_iteration_of(&[452]), vec![]);
    orch.ingest_once().await.unwrap();
    let repairs = orch.reconcile_dual_state_labels().await.unwrap();
    assert_eq!(
        repairs, 0,
        "a current-iteration member projects Ready, so its 'ready' label is canonical, not stale — stripping it here would flap the label a transition just pushed"
    );
    assert_eq!(adapter.pushed_events().len(), 0);
}

#[tokio::test]
async fn reconcile_marks_the_ready_label_stale_for_a_not_started_non_member() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(453, Work::NotStarted)]);
    adapter.seed_labels(453, vec!["ready".to_string(), "feature".to_string()]);
    let orch = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    let orch_dyn: Arc<dyn RefreshDriver> = orch.clone();
    bind_seeded_refresh(&orch_dyn, vec![], current_iteration_of(&[]), vec![]);
    orch.ingest_once().await.unwrap();
    let repairs = orch.reconcile_dual_state_labels().await.unwrap();
    assert_eq!(
        repairs, 1,
        "a not-started non-member projects Icebox, so its 'ready' label is stale and gets swept — the reconciler self-heals the mirror"
    );
    let pushed = adapter.pushed_events();
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::TagRemoved { label } if pushed[0].id == StoryId::new(453) && label == "ready"
    ));
}
