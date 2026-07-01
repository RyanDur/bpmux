#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::handle_retire_outcome;
use crate::product_management::board::adapters::github::{
    decode_iteration_from_story, decode_outcome_from_story,
};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::board::{IterationReader, OutcomeReader, RefreshDriver};
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::outcome::{Outcome, OutcomeAction, OutcomeId};
use crate::product_management::story::{BoardState, Story, StoryId, StoryKind};
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

fn seed(id: u64, title: &str, story_type: StoryKind, labels: &[&str]) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = labels.iter().map(ToString::to_string).collect();
    s.raw_labels.push(
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .encode_story_type(story_type),
    );
    s.story_type = story_type;
    s
}

fn seed_outcome(id: u64, title: &str, labels: &[&str]) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = labels.iter().map(ToString::to_string).collect();
    s
}

fn iteration_in_outcome(id: u64, outcome: u64) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["iteration".to_string()];
    s.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(outcome)),
            stories: vec![],
        }
        .write_into_body("## composed"),
    );
    s
}

fn outcome_with(id: u64, iterations: Vec<u64>) -> Story {
    let mut s = seed_outcome(id, "Outcome", &["outcome", "horizon.now"]);
    s.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: iterations.into_iter().map(IterationId::new).collect(),
        }
        .write_into_body("## Outcome\n"),
    );
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

async fn roadmap_numbers_for(outcomes: Vec<Outcome>) -> Vec<u64> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    orch.bind_outcome_reader(Arc::new(SeededOutcomes(outcomes)));
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
    let seed700 = seed_outcome(700, "Live", &["outcome", "horizon.now"]);
    let seed710 = seed_outcome(710, "Abandoned", &["outcome", "horizon.now"]);
    let refresh = build_refresh(vec![seed700.clone(), seed710.clone()]).await;
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
        roadmap_numbers_for(vec![decode_outcome_from_story(&seed700), retired.clone()]).await,
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
