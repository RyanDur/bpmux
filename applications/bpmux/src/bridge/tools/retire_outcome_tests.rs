#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::handle_retire_outcome;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::iteration::IterationId;
use crate::product_management::outcome::OutcomeId;
use crate::product_management::story::{Story, StoryId, StoryKind, StoryState, StoryType};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn seed(id: u64, title: &str, story_type: StoryType, labels: &[&str]) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        StoryState::Icebox,
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

fn iteration_in_outcome(id: u64, outcome: u64) -> Story {
    let mut s = seed(id, &format!("Iteration #{id}"), StoryType::Iteration, &[]);
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
    let mut s = seed(
        id,
        "Outcome",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    );
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
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    orch
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
    let refresh = build_refresh(vec![
        seed(700, "Live", StoryType::Outcome, &["outcome", "horizon.now"]),
        seed(
            710,
            "Abandoned",
            StoryType::Outcome,
            &["outcome", "horizon.now"],
        ),
    ])
    .await;
    assert_eq!(
        band_numbers(&refresh),
        vec![700, 710],
        "both outcomes visible before retire"
    );

    let result = handle_retire_outcome(
        &args(710, Some("superseded by #700")),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("pm retires the abandoned outcome");

    assert!(
        refresh
            .outcomes()
            .iter()
            .find(|o| o.id().saved_id() == Some(StoryId::new(710)))
            .expect("present")
            .is_retired(),
        "BOTH-HALVES half 1: the domain state mutated to Retired"
    );
    assert_eq!(
        band_numbers(&refresh),
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
    let refresh = build_refresh(vec![seed(
        700,
        "X",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    let err = handle_retire_outcome(&args(700, None), "pm:claude-code", Some(&refresh))
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
            .find(|o| o.id().saved_id() == Some(StoryId::new(700)))
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
    let result = handle_retire_outcome(&args(700, Some("pivot")), "pm:claude-code", Some(&refresh))
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
    let refresh = build_refresh(vec![seed(
        700,
        "X",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    let err = handle_retire_outcome(&args(700, Some("r")), "dev:claude-code", Some(&refresh))
        .await
        .expect_err("dev cannot retire an outcome (pm-lane)");
    assert!(err.contains("pm"), "the refusal names the pm-lane: {err}");
}

#[tokio::test]
async fn retire_aimed_at_a_non_outcome_teaches_the_boundary_1761() {
    let refresh = build_refresh(vec![seed(
        700,
        "A story",
        StoryType::Story(StoryKind::Feature),
        &["enhancement"],
    )])
    .await;
    let err = handle_retire_outcome(&args(700, Some("r")), "pm:claude-code", Some(&refresh))
        .await
        .expect_err("retire aimed at a non-outcome is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary instead of closing a plain story: {err}"
    );
}
