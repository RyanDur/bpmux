#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::bridge::dispatch::{dispatch_list_ready, dispatch_move_story};
use crate::product_management::board::adapters::github::decode_iteration_from_story;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::{
    BoardStore, IterationReader, RefreshDriver, SyncOrchestrator,
};
use crate::product_management::iteration::{Iteration, IterationAction};
use crate::product_management::story::{Story, StoryId, StoryState};
use crate::store::{AppAction, Dispatch};
use chrono::{TimeZone, Utc};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Clone)]
struct SharedIterations(Arc<Mutex<Vec<Iteration>>>);
impl IterationReader for SharedIterations {
    fn iterations(&self) -> Vec<Iteration> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

fn merge_dispatched(shared: &Arc<Mutex<Vec<Iteration>>>, rx: &mut UnboundedReceiver<AppAction>) {
    let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Iteration(IterationAction::Upserted(items)) = action {
            for item in items {
                match guard.iter().position(|existing| existing.id == item.id) {
                    Some(index) => guard[index] = item,
                    None => guard.push(item),
                }
            }
        }
    }
}

fn ready_story(id: u64) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("story-{id}"),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn current_iteration_story(id: u64) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.raw_labels.push("iteration".to_string());
    s.raw_labels.push("current-iteration".to_string());
    s
}

async fn build_orchestrator(
    stories: Vec<Story>,
) -> (Arc<dyn RefreshDriver>, Arc<Mutex<Vec<Iteration>>>) {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories.clone());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    let iterations: Vec<Iteration> = stories
        .iter()
        .filter(|s| {
            s.raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case("iteration"))
        })
        .map(decode_iteration_from_story)
        .collect();
    let shared = Arc::new(Mutex::new(iterations));
    orch.bind_iteration_reader(Arc::new(SharedIterations(shared.clone())));
    (orch, shared)
}

async fn move_to_ready_position(
    orch: &Arc<dyn RefreshDriver>,
    shared: &Arc<Mutex<Vec<Iteration>>>,
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
    merge_dispatched(shared, &mut rx);
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
    let (orch, shared) = build_orchestrator(vec![
        current_iteration_story(7000),
        ready_story(5001),
        ready_story(5002),
        ready_story(5003),
        ready_story(5004),
    ])
    .await;

    move_to_ready_position(&orch, &shared, 5001, 1).await;
    move_to_ready_position(&orch, &shared, 5002, 1).await;
    move_to_ready_position(&orch, &shared, 5003, 1).await;
    move_to_ready_position(&orch, &shared, 5004, 1).await;

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
    let (orch, shared) = build_orchestrator(vec![
        current_iteration_story(7000),
        ready_story(5002),
        ready_story(5001),
    ])
    .await;

    move_to_ready_position(&orch, &shared, 5002, 1).await;

    let queue = ready_queue_ids(&orch).await;
    assert_eq!(
        queue,
        vec![5002, 5001],
        "#1579: a positioned current-iteration member (5002) sorts before a loose ready \
         non-member (5001) even though its issue-id is higher; loose-ready falls to \
         issue-id after members; got {queue:?}"
    );
}
