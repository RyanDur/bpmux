#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::{handle_create_outcome, handle_map_iteration_to_outcome};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::iteration::IterationId;
use crate::product_management::outcome::OutcomeId;
use crate::product_management::story::{Horizon, Story, StoryId, StoryState, StoryType};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;

async fn build_empty_refresh() -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    orch
}

fn typed_seed(number: u64, story_type: StoryType, label: &str) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(number),
        format!("#{number}"),
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

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
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
    let result = handle_create_outcome(
        &args("The team runs on its own infra", "next"),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("pm can create an outcome");
    assert!(result.contains("outcome created:"), "result: {result}");
    let outcomes = refresh.outcomes();
    let outcome = outcomes
        .iter()
        .find(|o| o.content.title == "The team runs on its own infra")
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
    let err = handle_create_outcome(&args("X", "now"), "dev:claude-code", Some(&refresh))
        .await
        .expect_err("dev cannot create outcomes (pm-lane)");
    assert!(err.contains("pm-lane"), "refusal names pm-lane: {err}");
}

#[tokio::test]
async fn create_outcome_rejects_invalid_horizon_1738() {
    let refresh = build_empty_refresh().await;
    let err = handle_create_outcome(&args("X", "someday"), "pm:claude-code", Some(&refresh))
        .await
        .expect_err("an invalid horizon is rejected");
    assert!(
        err.contains("invalid horizon"),
        "refusal explains the horizon must be now/next/later: {err}"
    );
}

#[tokio::test]
async fn pm_maps_an_iteration_to_an_outcome_in_order_1738() {
    let refresh = build_refresh(vec![
        typed_seed(700, StoryType::Outcome, "outcome"),
        typed_seed(710, StoryType::Iteration, "iteration"),
    ])
    .await;
    handle_map_iteration_to_outcome(&map_args(700, 710, 1), "pm:claude-code", Some(&refresh))
        .await
        .expect("pm maps the iteration to the outcome");
    let outcome = refresh
        .outcomes()
        .into_iter()
        .find(|outcome| outcome.id().saved_id() == Some(StoryId::new(700)))
        .expect("outcome present");
    assert_eq!(
        outcome.iterations,
        vec![IterationId::new(710)],
        "the outcome holds its ordered iteration list (parent-holds-order)"
    );
    assert_eq!(
        refresh
            .iteration(IterationId::new(710))
            .expect("iteration present")
            .builds_outcome(),
        Some(OutcomeId::new(700)),
        "and the iteration carries the back-FK to the outcome it builds — the drill-mapping"
    );
}

#[tokio::test]
async fn map_iteration_to_outcome_is_pm_lane_non_pm_refused_1738() {
    let refresh = build_refresh(vec![
        typed_seed(700, StoryType::Outcome, "outcome"),
        typed_seed(710, StoryType::Iteration, "iteration"),
    ])
    .await;
    let err =
        handle_map_iteration_to_outcome(&map_args(700, 710, 1), "dev:claude-code", Some(&refresh))
            .await
            .expect_err("dev cannot map (pm-lane)");
    assert!(err.contains("pm"), "refusal names pm-lane: {err}");
}

#[tokio::test]
async fn map_rejects_non_outcome_target_1738() {
    let refresh = build_refresh(vec![
        typed_seed(700, StoryType::Epic, "epic"),
        typed_seed(710, StoryType::Iteration, "iteration"),
    ])
    .await;
    let err =
        handle_map_iteration_to_outcome(&map_args(700, 710, 1), "pm:claude-code", Some(&refresh))
            .await
            .expect_err("an epic is not an outcome — iterations map only to outcomes");
    assert!(
        err.contains("not an Outcome"),
        "refusal explains the target must be an Outcome: {err}"
    );
}
