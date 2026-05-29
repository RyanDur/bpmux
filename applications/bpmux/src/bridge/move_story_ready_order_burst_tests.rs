#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::board::BoardStore;
use crate::board::RefreshDriver;
use crate::board::SyncOrchestrator;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState};
use crate::bridge::dispatch::{dispatch_test_list_ready, dispatch_test_move_story};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn ready_story(number: u64) -> Story {
    Story::new_minimal(
        number,
        format!("story-{number}"),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn current_iteration_story(number: u64) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration #{number}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.story_type = crate::board::domain::StoryType::Iteration;
    s.is_current = true;
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

async fn move_to_ready_position(orch: &Arc<dyn RefreshDriver>, number: u64, position: u64) {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("target_state".to_string(), serde_json::json!("ready"));
    args.insert("position".to_string(), serde_json::json!(position));
    args.insert(
        "rationale".to_string(),
        serde_json::json!("burst-regression #1579: distinct positions under rapid reorder"),
    );
    dispatch_test_move_story(args, "pm", Some(orch))
        .await
        .unwrap_or_else(|e| panic!("move_story(#{number}, pos {position}) must succeed: {e}"));
}

async fn ready_queue_numbers(orch: &Arc<dyn RefreshDriver>) -> Vec<u64> {
    let raw = dispatch_test_list_ready(Some(orch)).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    parsed
        .as_array()
        .expect("list_ready returns an array")
        .iter()
        .map(|row| {
            row["number"]
                .as_u64()
                .expect("each row has a numeric number")
        })
        .collect()
}

#[tokio::test]
async fn burst_reorder_lands_each_story_at_its_own_position() {
    let orch = build_orchestrator(vec![
        current_iteration_story(7000),
        ready_story(5001),
        ready_story(5002),
        ready_story(5003),
        ready_story(5004),
    ])
    .await;

    move_to_ready_position(&orch, 5001, 4).await;
    move_to_ready_position(&orch, 5002, 3).await;
    move_to_ready_position(&orch, 5003, 2).await;
    move_to_ready_position(&orch, 5004, 1).await;

    let queue = ready_queue_numbers(&orch).await;
    assert_eq!(
        queue,
        vec![5004, 5003, 5002, 5001],
        "#1579: a burst of move_story(ready, position) ops must land each story at its own \
         requested position (reverse of issue-number here), not collapse to one position and \
         fall back to numeric sort; got {queue:?}"
    );
}

#[tokio::test]
async fn loose_ready_non_members_sort_after_positioned_members_by_issue_number() {
    let orch = build_orchestrator(vec![
        current_iteration_story(7000),
        ready_story(5002),
        ready_story(5001),
    ])
    .await;

    move_to_ready_position(&orch, 5002, 1).await;

    let queue = ready_queue_numbers(&orch).await;
    assert_eq!(
        queue,
        vec![5002, 5001],
        "#1579: a positioned current-iteration member (5002) sorts before a loose ready \
         non-member (5001) even though its issue-number is higher; loose-ready falls to \
         issue-number after members; got {queue:?}"
    );
}
