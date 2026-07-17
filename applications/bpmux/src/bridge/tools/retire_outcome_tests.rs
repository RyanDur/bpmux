#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::handle_retire_outcome;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, Dispatch};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use product_management::iteration::{Iteration, IterationId};
use product_management::outcome::{Outcome, OutcomeAction, OutcomeId};
use product_management::story::{Story, StoryId, StoryKind, Work};
use serde_json::{Map, Value, json};
use source_codec::mapper::{decode_iteration_from_story, decode_outcome_from_story};
use source_codec::relationships::{IterationRelationships, OutcomeRelationships, Relationships};
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

fn seed(id: u64, title: &str, story_type: StoryKind, labels: &[&str]) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = story_type;
    let mut seed_labels: Vec<String> = labels.iter().map(ToString::to_string).collect();
    seed_labels.push(source_codec::gh_label_codec::GhLabelCodec.encode_story_type(story_type));
    (s, seed_labels)
}

fn seed_outcome(id: u64, title: &str, labels: &[&str]) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, labels.iter().map(ToString::to_string).collect())
}

fn iteration_in_outcome(id: u64, outcome: u64) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        Work::NotStarted,
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

async fn build_refresh(
    seeds: Vec<(Story, Vec<String>)>,
) -> (
    crate::store::SeededStore,
    Arc<dyn RefreshDriver>,
    crate::store::Select,
) {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
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
    let store = crate::store::SeededStore::new(seeded, iterations, vec![]);
    store.bind_refresh(&orch);
    orch.refresh_all().await.unwrap();
    let select = store.select();
    (store, orch, select)
}

async fn roadmap_numbers_for(outcomes: Vec<Outcome>) -> Vec<u64> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let select = crate::store::bind_seeded_refresh(&orch, outcomes, vec![], vec![]);
    orch.refresh_all().await.unwrap();
    band_numbers(&select)
}

fn args(outcome: u64, reason: Option<&str>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("outcome_id".into(), json!(outcome));
    if let Some(re) = reason {
        m.insert("reason".into(), json!(re));
    }
    m
}

fn band_numbers(select: &crate::store::Select) -> Vec<u64> {
    product_management::roadmap::selectors::roadmap_cards(
        &select.select(|state| product_management::outcome::all(&state.outcomes)),
        &select.select(|state| product_management::iteration::all(&state.iterations)),
        workflow::current_workflow(),
    )
    .into_iter()
    .map(|card| card.id.value())
    .collect()
}

#[tokio::test]
async fn retire_mutates_the_domain_and_the_outcome_drops_from_the_roadmap_view_1761() {
    let (live, live_labels) = seed_outcome(700, "Live", &["outcome", "horizon.now"]);
    let (_store, refresh, select) = build_refresh(vec![
        (live.clone(), live_labels.clone()),
        seed_outcome(710, "Abandoned", &["outcome", "horizon.now"]),
    ])
    .await;
    assert_eq!(
        band_numbers(&select),
        vec![700, 710],
        "both outcomes visible before retire"
    );

    let (dispatch, mut rx) = dispatch_channel();
    let result = handle_retire_outcome(
        &args(710, Some("superseded by #700")),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
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
    let (_store, refresh, select) =
        build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_retire_outcome(
        &args(700, None),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect_err("retire is destructive — a reason is required");
    assert!(
        err.contains("reason"),
        "the error explains the reason is the audit-trail: {err}"
    );
    assert!(
        !select
            .select(|state| product_management::outcome::all(&state.outcomes))
            .iter()
            .find(|outcome| outcome.id == OutcomeId::new(700))
            .expect("present")
            .is_retired(),
        "a reasonless retire mutates nothing"
    );
}

#[tokio::test]
async fn retire_states_the_orphaned_iteration_mapping_count_1761() {
    let (_store, refresh, select) = build_refresh(vec![
        outcome_with(700, vec![710, 720]),
        iteration_in_outcome(710, 700),
        iteration_in_outcome(720, 700),
    ])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let result = handle_retire_outcome(
        &args(700, Some("pivot")),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect("pm retires an outcome with mapped iterations");
    assert!(
        result.contains("2 iteration"),
        "the confirm states the orphaned-mapping count so the PM can re-home them: {result}"
    );
    assert!(
        select
            .select(|state| {
                product_management::iteration::get(&state.iterations, IterationId::new(710))
            })
            .is_some_and(|iteration| !iteration.complete),
        "retire is soft-delete on the OUTCOME — its iterations are NOT closed (re-mappable)"
    );
}

#[tokio::test]
async fn retire_outcome_is_pm_lane_non_pm_refused_1761() {
    let (_store, refresh, select) =
        build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_retire_outcome(
        &args(700, Some("r")),
        "dev:claude-code",
        "dev",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect_err("dev cannot retire an outcome (pm-lane)");
    assert!(err.contains("pm"), "the refusal names the pm-lane: {err}");
}

#[tokio::test]
async fn retire_aimed_at_a_non_outcome_teaches_the_boundary_1761() {
    let (_store, refresh, select) = build_refresh(vec![seed(
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
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect_err("retire aimed at a non-outcome is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary instead of closing a plain story: {err}"
    );
}
