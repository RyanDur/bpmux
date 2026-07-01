#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::{handle_create_outcome, handle_map_iteration_to_outcome};
use crate::product_management::board::adapters::github::{
    decode_iteration_from_story, decode_outcome_from_story,
};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::board::{IterationReader, OutcomeReader, RefreshDriver};
use crate::product_management::iteration::{Iteration, IterationAction, IterationId};
use crate::product_management::outcome::{Outcome, OutcomeAction, OutcomeId};
use crate::product_management::story::{Horizon, Story, StoryId, StoryKind, StoryState, StoryType};
use crate::store::{AppAction, Dispatch};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

struct SeededOutcomes(Vec<Outcome>);
impl OutcomeReader for SeededOutcomes {
    fn outcomes(&self) -> Vec<Outcome> {
        self.0.clone()
    }
}

struct SeededIterations(Vec<Iteration>);
impl IterationReader for SeededIterations {
    fn iterations(&self) -> Vec<Iteration> {
        self.0.clone()
    }
}

fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
    (Dispatch::new(tx), rx)
}

fn upserted(rx: &mut UnboundedReceiver<AppAction>) -> Vec<Outcome> {
    let mut found = Vec::new();
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Outcome(OutcomeAction::Upserted(outcomes)) = action {
            found = outcomes;
        }
    }
    found
}

fn seed_iteration(id: u64) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["iteration".to_string()];
    s
}

async fn build_empty_refresh() -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    orch
}

fn typed_seed(id: u64, story_type: StoryType, label: &str) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("#{id}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec![label.to_string()];
    s.raw_labels.push(
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .encode_story_type(story_type),
    );
    s.story_type = story_type;
    s
}

fn seed_outcome(id: u64) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("#{id}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["outcome".to_string()];
    s
}

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories.clone());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    let seeded: Vec<Outcome> = stories
        .iter()
        .filter(|s| {
            s.raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case("outcome"))
        })
        .map(decode_outcome_from_story)
        .collect();
    orch.bind_outcome_reader(Arc::new(SeededOutcomes(seeded)));
    let iterations: Vec<Iteration> = stories
        .iter()
        .filter(|s| {
            s.raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case("iteration"))
        })
        .map(decode_iteration_from_story)
        .collect();
    orch.bind_iteration_reader(Arc::new(SeededIterations(iterations)));
    orch
}

fn map_args(outcome: u64, iteration: u64, position: u64) -> Map<String, Value> {
    json!({ "outcome_id": outcome, "iteration_id": iteration, "position": position })
        .as_object()
        .cloned()
        .unwrap_or_default()
}

fn args(title: &str, horizon: &str) -> Map<String, Value> {
    json!({ "title": title, "horizon": horizon })
        .as_object()
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn pm_creates_a_first_class_outcome_distinct_from_epic_with_horizon_1738() {
    let refresh = build_empty_refresh().await;
    let (dispatch, mut rx) = dispatch_channel();
    let result = handle_create_outcome(
        &args("The team runs on its own infra", "next"),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect("pm can create an outcome");
    assert!(result.contains("outcome created:"), "result: {result}");
    let outcomes = upserted(&mut rx);
    let outcome = outcomes
        .iter()
        .find(|o| o.title == "The team runs on its own infra")
        .expect(
            "the outcome is a first-class Outcome entity, NOT an Epic — no epic≡outcome conflation",
        );
    assert!(
        !refresh
            .all_stories()
            .iter()
            .any(|s| s.title == "The team runs on its own infra"),
        "an outcome must never be modelled as a story/epic — it lives in the outcome collection, never the board's story set"
    );
    assert_eq!(
        outcome.horizon(),
        Some(Horizon::Next),
        "the outcome carries its horizon, derived from the recorded horizon label"
    );
}

#[tokio::test]
async fn create_outcome_is_pm_lane_non_pm_refused_1738() {
    let refresh = build_empty_refresh().await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_create_outcome(
        &args("X", "now"),
        "dev:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect_err("dev cannot create outcomes (pm-lane)");
    assert!(err.contains("pm-lane"), "refusal names pm-lane: {err}");
}

#[tokio::test]
async fn create_outcome_rejects_invalid_horizon_1738() {
    let refresh = build_empty_refresh().await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_create_outcome(
        &args("X", "someday"),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect_err("an invalid horizon is rejected");
    assert!(
        err.contains("invalid horizon"),
        "refusal explains the horizon must be now/next/later: {err}"
    );
}

#[tokio::test]
async fn pm_maps_an_iteration_to_an_outcome_in_order_1738() {
    let refresh = build_refresh(vec![seed_outcome(700), seed_iteration(710)]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_map_iteration_to_outcome(
        &map_args(700, 710, 1),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect("pm maps the iteration to the outcome");
    let mut outcomes = Vec::new();
    let mut iterations = Vec::new();
    while let Ok(action) = rx.try_recv() {
        match action {
            AppAction::Outcome(OutcomeAction::Upserted(o)) => outcomes = o,
            AppAction::Iteration(IterationAction::Upserted(i)) => iterations = i,
            _ => {}
        }
    }
    let outcome = outcomes
        .iter()
        .find(|outcome| outcome.id == OutcomeId::new(700))
        .expect("outcome present in the optimistic dispatch");
    assert_eq!(
        outcome.iterations,
        vec![IterationId::new(710)],
        "the outcome holds its ordered iteration list (parent-holds-order)"
    );
    let iteration = iterations
        .iter()
        .find(|it| it.id == IterationId::new(710))
        .expect("iteration present in the optimistic dispatch");
    assert_eq!(
        iteration.builds_outcome(),
        Some(OutcomeId::new(700)),
        "and the iteration carries the back-FK to the outcome it builds — the drill-mapping"
    );
}

#[tokio::test]
async fn map_iteration_to_outcome_is_pm_lane_non_pm_refused_1738() {
    let refresh = build_refresh(vec![seed_outcome(700), seed_iteration(710)]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_map_iteration_to_outcome(
        &map_args(700, 710, 1),
        "dev:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect_err("dev cannot map (pm-lane)");
    assert!(err.contains("pm"), "refusal names pm-lane: {err}");
}

#[tokio::test]
async fn map_rejects_non_outcome_target_1738() {
    let refresh = build_refresh(vec![
        typed_seed(700, StoryType::Story(StoryKind::Feature), "feature"),
        seed_iteration(710),
    ])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_map_iteration_to_outcome(
        &map_args(700, 710, 1),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect_err("a feature is not an outcome — iterations map only to outcomes");
    assert!(
        err.contains("not an Outcome"),
        "refusal explains the target must be an Outcome: {err}"
    );
}
