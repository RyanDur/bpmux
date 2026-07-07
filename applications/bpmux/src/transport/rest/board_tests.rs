#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::board::{board_iterations, board_roadmap};
use crate::integrations::github::mapper::{decode_iteration_from_story, decode_outcome_from_story};
use crate::integrations::github::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::outcome::OutcomeId;
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn iteration_seed(id: u64, title: &str) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, vec!["iteration".to_string()])
}

fn seed_outcome(id: u64, title: &str, labels: &[&str]) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, labels.iter().map(ToString::to_string).collect())
}

async fn build_refresh(seeds: Vec<(Story, Vec<String>)>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let seeded: Vec<_> = seeds
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

#[tokio::test]
async fn board_roadmap_serves_flat_priority_cards_carrying_the_enriched_state() {
    let refresh = build_refresh(vec![
        seed_outcome(710, "Later thing", &["outcome", "horizon.later"]),
        seed_outcome(700, "Now thing", &["outcome", "horizon.now"]),
    ])
    .await;

    let cards = board_roadmap(&refresh);

    assert_eq!(cards.len(), 2, "both non-retired outcomes reach the wire");
    assert_eq!(
        cards[0]["id"], 700,
        "the served layer is flat in priority order — Now precedes Later"
    );
    assert_eq!(cards[1]["id"], 710);
    assert_eq!(
        cards[0]["state"], "planned",
        "the enriched kebab state vocabulary rides the wire (an outcome with no building iterations reads planned)"
    );
    assert_eq!(cards[0]["tone"], "now");
    assert_eq!(cards[1]["tone"], "future");
    assert_eq!(cards[0]["horizon"], "now");
    assert!(
        cards[0].get("iterations").is_none(),
        "the roadmap layer is flat — iterations are layer 2, fetched by outcome id, never saturating the card: {}",
        cards[0]
    );
}

#[tokio::test]
async fn board_iterations_carry_the_outcome_membership_join_key_for_the_drill() {
    let (mut iteration, iteration_labels) = iteration_seed(810, "Iteration");
    iteration.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(700)),
            stories: vec![],
        }
        .write_into_body("## composed"),
    );
    let (mut outcome, outcome_labels) = seed_outcome(700, "Outcome", &["outcome", "horizon.now"]);
    outcome.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: vec![IterationId::new(810)],
        }
        .write_into_body("## Outcome\n"),
    );
    let refresh = build_refresh(vec![
        (outcome, outcome_labels),
        (iteration, iteration_labels),
    ])
    .await;

    let iterations = board_iterations(&refresh);
    let mapped = iterations
        .iter()
        .find(|i| i["id"] == 810)
        .expect("the mapped iteration reaches the wire");

    assert_eq!(
        mapped["builds_outcome"], 700,
        "the iteration carries which outcome it builds — the client joins drill-in iterations by this key"
    );
    assert_eq!(
        mapped["outcome_position"], 1,
        "and its 1-based position within that outcome's ordered list, so the drill orders without a second round-trip"
    );
}

#[test]
fn board_columns_json_lists_the_configured_columns_in_order() {
    let columns = super::board::board_columns_json();
    let ids: Vec<&str> = columns.iter().filter_map(|c| c["id"].as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "icebox",
            "ready",
            "in-progress",
            "desk-check",
            "acceptance",
            "accepted",
            "retired"
        ],
        "the wire's columns are the config skeleton with the CONFIGURED middle stages in order"
    );
    let desk_check = columns
        .iter()
        .find(|c| c["id"] == "desk-check")
        .expect("desk-check column present under the team config");
    assert_eq!(
        desk_check["label"], "Desk Check",
        "labels render title-case from the stage id"
    );
}
