#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::bridge::dispatch::{dispatch_list_ready, dispatch_move_story};
use crate::product_management::board::adapters::github::decode_iteration_from_story;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::iteration::{Iteration, IterationAction};
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::store::{AppAction, Dispatch, SeededStore};
use crate::sync::{RefreshDriver, SyncOrchestrator};
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

fn merge_dispatched(seed: &SeededStore, rx: &mut UnboundedReceiver<AppAction>) {
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Iteration(IterationAction::Upserted(_)) = &action {
            seed.dispatch(action);
        }
    }
}

fn ready_story(id: u64) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("story-{id}"),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    (s, Vec::new())
}

fn current_iteration_story(id: u64) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    (
        s,
        vec!["iteration".to_string(), "current-iteration".to_string()],
    )
}

async fn build_orchestrator(
    seeds: Vec<(Story, Vec<String>)>,
) -> (Arc<dyn RefreshDriver>, SeededStore) {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let iterations: Vec<Iteration> = seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
        .map(|(story, labels)| decode_iteration_from_story(story, labels))
        .collect();
    let seed = SeededStore::new(vec![], iterations, vec![]);
    seed.bind_refresh(&orch);
    orch.refresh_all().await.unwrap();
    (orch, seed)
}

async fn move_to_ready_position(
    orch: &Arc<dyn RefreshDriver>,
    seed: &SeededStore,
    id: u64,
    position: u64,
) {
    let mut args = serde_json::Map::new();
    args.insert("id".to_string(), serde_json::json!(id));
    args.insert("target_state".to_string(), serde_json::json!("ready"));
    args.insert("position".to_string(), serde_json::json!(position));
    args.insert(
        "rationale".to_string(),
        serde_json::json!("burst-regression #1579: distinct positions under rapid reorder"),
    );
    let (dispatch, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
    let dispatch = Dispatch::new(dispatch);
    dispatch_move_story(args, "pm", Some(orch), &dispatch)
        .await
        .unwrap_or_else(|e| panic!("move_story(#{id}, pos {position}) must succeed: {e}"));
    merge_dispatched(seed, &mut rx);
}

async fn ready_queue_ids(orch: &Arc<dyn RefreshDriver>) -> Vec<u64> {
    let raw = dispatch_list_ready(Some(orch)).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    parsed
        .as_array()
        .expect("list_ready returns an array")
        .iter()
        .map(|row| row["id"].as_u64().expect("each row has a numeric id"))
        .collect()
}

#[tokio::test]
async fn burst_reorder_lands_each_story_at_its_own_position() {
    let (orch, seed) = build_orchestrator(vec![
        current_iteration_story(7000),
        ready_story(5001),
        ready_story(5002),
        ready_story(5003),
        ready_story(5004),
    ])
    .await;

    move_to_ready_position(&orch, &seed, 5001, 1).await;
    move_to_ready_position(&orch, &seed, 5002, 1).await;
    move_to_ready_position(&orch, &seed, 5003, 1).await;
    move_to_ready_position(&orch, &seed, 5004, 1).await;

    let queue = ready_queue_ids(&orch).await;
    assert_eq!(
        queue,
        vec![5004, 5003, 5002, 5001],
        "#1579: a burst of move_story(ready, position 1) ops each lands distinctly at the \
         front of the parent-held order (reverse of move order), proving the burst neither \
         collapses to one slot nor falls back to numeric sort; got {queue:?}"
    );
}

#[tokio::test]
async fn loose_ready_non_members_sort_after_positioned_members_by_issue_id() {
    let (orch, seed) = build_orchestrator(vec![
        current_iteration_story(7000),
        ready_story(5002),
        ready_story(5001),
    ])
    .await;

    move_to_ready_position(&orch, &seed, 5002, 1).await;

    let queue = ready_queue_ids(&orch).await;
    assert_eq!(
        queue,
        vec![5002, 5001],
        "#1579: a positioned current-iteration member (5002) sorts before a loose ready \
         non-member (5001) even though its issue-id is higher; loose-ready falls to \
         issue-id after members; got {queue:?}"
    );
}
