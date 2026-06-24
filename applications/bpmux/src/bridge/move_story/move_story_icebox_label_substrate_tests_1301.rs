use crate::bridge::tools_stories::reorder_icebox_story_via_labels;
use crate::product_management::board::BoardStore;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::SyncOrchestrator;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::domain::{Story, StoryState};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn seed_story_with_labels(number: u64, state: StoryState, labels: Vec<String>) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("story-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.raw_labels = labels;
    s
}

async fn build_orchestrator(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    orch
}

#[tokio::test]
async fn moving_loose_icebox_story_into_position_lands_in_unscheduled() {
    let orch = build_orchestrator(vec![
        seed_story_with_labels(3100, StoryState::Icebox, vec!["icebox".into()]),
        seed_story_with_labels(
            3101,
            StoryState::Icebox,
            vec!["iteration-Unscheduled.100".into()],
        ),
        seed_story_with_labels(
            3102,
            StoryState::Icebox,
            vec!["iteration-Unscheduled.200".into()],
        ),
    ])
    .await;
    reorder_icebox_story_via_labels(&orch, 3100, 1).unwrap();
    let s = orch.story(3100).unwrap();
    assert!(
        s.raw_labels.iter().any(|l| l == "iteration-Unscheduled.50"),
        "loose story moved into icebox-position-1 joins Unscheduled at head; got {:?}",
        s.raw_labels
    );
}

#[tokio::test]
async fn move_into_icebox_preserves_iteration_membership_when_story_already_in_named_container() {
    let orch = build_orchestrator(vec![seed_story_with_labels(
        3200,
        StoryState::Ready,
        vec!["iteration-Pegasus.100".into(), "ready".into()],
    )])
    .await;
    reorder_icebox_story_via_labels(&orch, 3200, 1).unwrap();
    let s = orch.story(3200).unwrap();
    assert!(
        s.raw_labels
            .iter()
            .any(|l| l.starts_with("iteration-Pegasus.")),
        "story keeps Pegasus theme membership after move-to-icebox at position 1: {:?}",
        s.raw_labels
    );
}
