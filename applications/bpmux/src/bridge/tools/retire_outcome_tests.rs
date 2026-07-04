#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::handle_retire_outcome;
use crate::integrations::github::mapper::{decode_iteration_from_story, decode_outcome_from_story};
use crate::integrations::github::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::outcome::{Outcome, OutcomeAction, OutcomeId};
use crate::product_management::story::{BoardState, Story, StoryId, StoryKind};
use crate::store::{AppAction, Dispatch};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

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

fn seed(id: u64, title: &str, story_type: StoryKind, labels: &[&str]) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = story_type;
    let mut seed_labels: Vec<String> = labels.iter().map(ToString::to_string).collect();
    seed_labels.push(
        crate::integrations::github::gh_label_codec::GhLabelCodec.encode_story_type(story_type),
    );
    (s, seed_labels)
}

fn seed_outcome(id: u64, title: &str, labels: &[&str]) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, labels.iter().map(ToString::to_string).collect())
}

fn iteration_in_outcome(id: u64, outcome: u64) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(outcome)),
            stories: vec![],
        }
        .write_into_body("## composed"),
    );
    (s, vec!["iteration".to_string()])
}

fn outcome_with(id: u64, iterations: Vec<u64>) -> (Story, Vec<String>) {
    let (mut s, labels) = seed_outcome(id, "Outcome", &["outcome", "horizon.now"]);
    s.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: iterations.into_iter().map(IterationId::new).collect(),
        }
        .write_into_body("## Outcome\n"),
    );
    (s, labels)
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
        .map(|(story, labels)| decode_iteration_from_story(story, labels))
        .collect();
    crate::store::bind_seeded_refresh(&orch, seeded, iterations, vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

async fn roadmap_numbers_for(outcomes: Vec<Outcome>) -> Vec<u64> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    crate::store::bind_seeded_refresh(&orch, outcomes, vec![], vec![]);
    orch.refresh_all().await.unwrap();
    band_numbers(&orch)
}

fn args(outcome: u64, reason: Option<&str>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("outcome_id".into(), json!(outcome));
    if let Some(re) = reason {
        m.insert("reason".into(), json!(re));
    }
    m
}

fn band_numbers(refresh: &Arc<dyn RefreshDriver>) -> Vec<u64> {
    crate::view::roadmap::roadmap_cards(refresh)
        .into_iter()
        .map(|c| c.id.value())
        .collect()
}

#[tokio::test]
async fn retire_mutates_the_domain_and_the_outcome_drops_from_the_roadmap_view_1761() {
    let (live, live_labels) = seed_outcome(700, "Live", &["outcome", "horizon.now"]);
    let refresh = build_refresh(vec![
        (live.clone(), live_labels.clone()),
        seed_outcome(710, "Abandoned", &["outcome", "horizon.now"]),
    ])
    .await;
    assert_eq!(
        band_numbers(&refresh),
        vec![700, 710],
        "both outcomes visible before retire"
    );

    let (dispatch, mut rx) = dispatch_channel();
    let result = handle_retire_outcome(
        &args(710, Some("superseded by #700")),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect("pm retires the abandoned outcome");

    let dispatched = upserted(&mut rx);
    let retired = dispatched
        .iter()
        .find(|o| o.id == OutcomeId::new(710))
        .expect("retire dispatches the outcome optimistically");
    assert!(
        retired.is_retired(),
        "BOTH-HALVES half 1: the domain state mutated to Retired"
    );
    assert_eq!(
        roadmap_numbers_for(vec![
            decode_outcome_from_story(&live, &live_labels),
            retired.clone()
        ])
        .await,
        vec![700],
        "BOTH-HALVES half 2: the retired outcome DROPS from bpmux roadmap (the same derive it serves) — retire isn't done until the view drops it"
    );
    assert!(
        result.contains("retired") && result.contains("superseded by #700"),
        "the confirm carries what it removed + the reason (stream audit): {result}"
    );
}

#[tokio::test]
async fn retire_with_no_reason_is_rejected_1761() {
    let refresh = build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_retire_outcome(
        &args(700, None),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect_err("retire is destructive — a reason is required");
    assert!(
        err.contains("reason"),
        "the error explains the reason is the audit-trail: {err}"
    );
    assert!(
        !refresh
            .outcomes()
            .iter()
            .find(|o| o.id == OutcomeId::new(700))
            .expect("present")
            .is_retired(),
        "a reasonless retire mutates nothing"
    );
}

#[tokio::test]
async fn retire_states_the_orphaned_iteration_mapping_count_1761() {
    let refresh = build_refresh(vec![
        outcome_with(700, vec![710, 720]),
        iteration_in_outcome(710, 700),
        iteration_in_outcome(720, 700),
    ])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let result = handle_retire_outcome(
        &args(700, Some("pivot")),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect("pm retires an outcome with mapped iterations");
    assert!(
        result.contains("2 iteration"),
        "the confirm states the orphaned-mapping count so the PM can re-home them: {result}"
    );
    assert!(
        refresh
            .iteration(IterationId::new(710))
            .is_some_and(|it| !it.complete),
        "retire is soft-delete on the OUTCOME — its iterations are NOT closed (re-mappable)"
    );
}

#[tokio::test]
async fn retire_outcome_is_pm_lane_non_pm_refused_1761() {
    let refresh = build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_retire_outcome(
        &args(700, Some("r")),
        "dev:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect_err("dev cannot retire an outcome (pm-lane)");
    assert!(err.contains("pm"), "the refusal names the pm-lane: {err}");
}

#[tokio::test]
async fn retire_aimed_at_a_non_outcome_teaches_the_boundary_1761() {
    let refresh = build_refresh(vec![seed(
        700,
        "A story",
        StoryKind::Feature,
        &["enhancement"],
    )])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_retire_outcome(
        &args(700, Some("r")),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect_err("retire aimed at a non-outcome is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary instead of closing a plain story: {err}"
    );
}
