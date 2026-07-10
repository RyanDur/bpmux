#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::{handle_create_outcome, handle_map_iteration_to_outcome};
use crate::integrations::github::mapper::{decode_iteration_from_story, decode_outcome_from_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::iteration::{Iteration, IterationAction, IterationId};
use crate::product_management::outcome::{Outcome, OutcomeAction, OutcomeId};
use crate::product_management::story::{Horizon, Story, StoryId, StoryKind, Work};
use crate::store::{AppAction, Dispatch};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
    Dispatch::recording()
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

fn seed_iteration(id: u64) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, vec!["iteration".to_string()])
}

async fn build_empty_refresh() -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

fn typed_seed(id: u64, story_type: StoryKind, label: &str) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("#{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = story_type;
    let labels = vec![
        label.to_string(),
        crate::integrations::github::gh_label_codec::GhLabelCodec.encode_story_type(story_type),
    ];
    (s, labels)
}

fn seed_outcome(id: u64) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("#{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, vec!["outcome".to_string()])
}

async fn build_refresh(seeds: Vec<(Story, Vec<String>)>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let seeded: Vec<Outcome> = seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("outcome")))
        .map(|(story, labels)| decode_outcome_from_story(story, labels))
        .collect();
    let iterations: Vec<Iteration> = seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
        .map(|(story, labels)| decode_iteration_from_story(story, labels, false))
        .collect();
    crate::store::bind_seeded_refresh(&orch, seeded, iterations, vec![]);
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
    let (dispatch, mut rx) = dispatch_channel();
    let result = handle_create_outcome(
        &args("The team runs on its own infra", "next"),
        "pm:claude-code",
        "pm",
        None,
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
        "dev",
        None,
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
        "pm",
        None,
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
    handle_map_iteration_to_outcome(&map_args(700, 710, 1), "pm", Some(&refresh), &dispatch)
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
    let err =
        handle_map_iteration_to_outcome(&map_args(700, 710, 1), "dev", Some(&refresh), &dispatch)
            .await
            .expect_err("dev cannot map (pm-lane)");
    assert!(err.contains("pm"), "refusal names pm-lane: {err}");
}

#[tokio::test]
async fn map_rejects_non_outcome_target_1738() {
    let refresh = build_refresh(vec![
        typed_seed(700, StoryKind::Feature, "feature"),
        seed_iteration(710),
    ])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let err =
        handle_map_iteration_to_outcome(&map_args(700, 710, 1), "pm", Some(&refresh), &dispatch)
            .await
            .expect_err("a feature is not an outcome — iterations map only to outcomes");
    assert!(
        err.contains("not an Outcome"),
        "refusal explains the target must be an Outcome: {err}"
    );
}
