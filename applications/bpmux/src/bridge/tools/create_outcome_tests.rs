#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::board::RefreshDriver;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Horizon, Story, StoryType};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::store::BoardStore;
use crate::bridge::tools_stories::handle_create_outcome;
use serde_json::{Map, Value, json};
use std::sync::Arc;

async fn build_empty_refresh() -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    orch
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
    let stories = refresh.all_stories();
    let outcome: &Story = stories
        .iter()
        .find(|s| s.title == "The team runs on its own infra")
        .expect("outcome issue persisted");
    assert_eq!(
        outcome.story_type,
        StoryType::Outcome,
        "the outcome is a first-class Outcome entity, NOT an Epic — no epic≡outcome conflation"
    );
    assert_ne!(
        outcome.story_type,
        StoryType::Epic,
        "an outcome must never be modelled as an epic"
    );
    assert_eq!(
        outcome.horizon,
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
