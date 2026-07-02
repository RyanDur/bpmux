use crate::bridge::tools_stories::reorder_icebox_story_via_labels;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::SyncOrchestrator;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::story::{BoardState, Story, StoryId};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn seed_story_with_labels(id: u64, state: BoardState, labels: Vec<String>) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("story-{id}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    (s, labels)
}

async fn build_orchestrator(seeds: Vec<(Story, Vec<String>)>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

#[tokio::test]
async fn moving_loose_icebox_story_into_position_lands_in_unscheduled() {
    let orch = build_orchestrator(vec![
        seed_story_with_labels(3100, BoardState::Icebox, vec!["icebox".into()]),
        seed_story_with_labels(
            3101,
            BoardState::Icebox,
            vec!["iteration-Unscheduled.100".into()],
        ),
        seed_story_with_labels(
            3102,
            BoardState::Icebox,
            vec!["iteration-Unscheduled.200".into()],
        ),
    ])
    .await;
    reorder_icebox_story_via_labels(&orch, 3100, 1).unwrap();
    let s = orch.story(3100).unwrap();
    assert_eq!(
        s.iteration_slot(),
        Some(&crate::product_management::story::IterationSlot {
            name: "Unscheduled".to_string(),
            position: 50,
        }),
        "loose story moved into icebox-position-1 joins Unscheduled at head; got {:?}",
        s.iteration_slot()
    );
}

#[tokio::test]
async fn move_into_icebox_preserves_iteration_membership_when_story_already_in_named_container() {
    let orch = build_orchestrator(vec![seed_story_with_labels(
        3200,
        BoardState::Ready,
        vec!["iteration-Pegasus.100".into(), "ready".into()],
    )])
    .await;
    reorder_icebox_story_via_labels(&orch, 3200, 1).unwrap();
    let s = orch.story(3200).unwrap();
    assert_eq!(
        s.iteration_slot().map(|slot| slot.name.as_str()),
        Some("Pegasus"),
        "story keeps Pegasus name membership after move-to-icebox at position 1: {:?}",
        s.iteration_slot()
    );
}
