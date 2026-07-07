#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::bridge::dispatch::dispatch_move_story;
use crate::integrations::github::mapper::decode_iteration_from_story;
use crate::integrations::in_memory::InMemorySourceAdapter;
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

fn icebox_story(id: u64) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("story-{id}"),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 7, 7, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    (s, Vec::new())
}

fn current_iteration_story(id: u64) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 7, 7, 0, 0, 0).unwrap(),
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
        .map(|(story, labels)| decode_iteration_from_story(story, labels, false))
        .collect();
    let seed = SeededStore::new(vec![], iterations, vec![]);
    seed.bind_refresh(&orch);
    orch.refresh_all().await.unwrap();
    (orch, seed)
}

async fn move_story(
    orch: &Arc<dyn RefreshDriver>,
    seed: &SeededStore,
    id: u64,
    target: &str,
    position: Option<u64>,
) -> Result<String, String> {
    let mut args = serde_json::Map::new();
    args.insert("id".to_string(), serde_json::json!(id));
    args.insert("target_state".to_string(), serde_json::json!(target));
    if let Some(pos) = position {
        args.insert("position".to_string(), serde_json::json!(pos));
    }
    args.insert(
        "rationale".to_string(),
        serde_json::json!("R-3.1: scheduling is current-iteration membership"),
    );
    let (dispatch, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
    let dispatch = Dispatch::new(dispatch);
    let result = dispatch_move_story(args, "pm", Some(orch), &dispatch).await;
    merge_dispatched(seed, &mut rx);
    result
}

fn current_members(orch: &Arc<dyn RefreshDriver>) -> Vec<u64> {
    orch.iterations()
        .into_iter()
        .find(|it| it.is_current)
        .map(|it| it.stories.iter().map(|s| s.value()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn a_bare_move_to_ready_joins_the_current_iteration_at_the_end() {
    let (orch, seed) = build_orchestrator(vec![
        current_iteration_story(7000),
        icebox_story(5001),
        icebox_story(5002),
    ])
    .await;

    move_story(&orch, &seed, 5001, "ready", Some(1))
        .await
        .expect("positioned ready move seeds the queue");
    move_story(&orch, &seed, 5002, "ready", None)
        .await
        .expect("a bare ready move succeeds when a current iteration exists");

    assert_eq!(
        current_members(&orch),
        vec![5001, 5002],
        "scheduling to ready IS joining the current iteration — the bare move appends"
    );
}

#[tokio::test]
async fn a_bare_move_to_ready_keeps_an_existing_members_position() {
    let (orch, seed) = build_orchestrator(vec![
        current_iteration_story(7000),
        icebox_story(5001),
        icebox_story(5002),
    ])
    .await;

    move_story(&orch, &seed, 5001, "ready", Some(1))
        .await
        .expect("member one takes the front");
    move_story(&orch, &seed, 5002, "ready", Some(2))
        .await
        .expect("member two takes the back");
    move_story(&orch, &seed, 5001, "in-progress", None)
        .await
        .expect("the front member starts work");
    move_story(&orch, &seed, 5001, "ready", None)
        .await
        .expect("the rework move back to ready succeeds");

    assert_eq!(
        current_members(&orch),
        vec![5001, 5002],
        "a story already scheduled keeps its slot on a bare re-entry — rework does not shove it to the back"
    );
}

#[tokio::test]
async fn a_move_to_ready_without_a_current_iteration_is_refused_before_mutating() {
    let (orch, seed) = build_orchestrator(vec![icebox_story(5001)]).await;

    let err = move_story(&orch, &seed, 5001, "ready", None)
        .await
        .expect_err("ready is current-iteration membership; with no current iteration there is nothing to join");

    assert!(
        err.contains("current iteration") && err.contains("designate"),
        "the refusal names the missing designation and the way out; got {err}"
    );
    assert_eq!(
        orch.story(5001).expect("story stays visible").state(),
        BoardState::Icebox,
        "the refusal happens in the validation phase — no state mutation leaked"
    );
}

#[tokio::test]
async fn a_move_to_icebox_removes_the_story_from_its_iteration() {
    let (orch, seed) =
        build_orchestrator(vec![current_iteration_story(7000), icebox_story(5001)]).await;

    move_story(&orch, &seed, 5001, "ready", Some(1))
        .await
        .expect("schedule the story first");
    assert_eq!(current_members(&orch), vec![5001]);

    move_story(&orch, &seed, 5001, "icebox", None)
        .await
        .expect("the icebox move succeeds");

    assert_eq!(
        current_members(&orch),
        Vec::<u64>::new(),
        "icebox means unscheduled — the move removes the story from the iteration (no more 'Icebox in an iteration')"
    );
    assert_eq!(
        orch.story(5001).expect("story stays visible").state(),
        BoardState::Icebox,
        "the story itself returns to icebox"
    );
}
