#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::board::{board_iterations, board_roadmap, sort_ready_by_membership};
use chrono::{TimeZone, Utc};
use integrations::in_memory::InMemorySourceAdapter;
use product_management::iteration::{Iteration, IterationId};
use product_management::outcome::OutcomeId;
use product_management::story::{Story, StoryId, Work};
use source_codec::mapper::{decode_iteration_from_story, decode_outcome_from_story};
use source_codec::relationships::{IterationRelationships, OutcomeRelationships, Relationships};
use std::sync::Arc;
use sync::SyncOrchestrator;

fn iteration_seed(id: u64, title: &str) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, vec!["iteration".to_string()])
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

async fn seeded_store(seeds: Vec<(Story, Vec<String>)>) -> store_test_support::SeededStore {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
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
    let store = store_test_support::SeededStore::new(seeded, iterations, vec![]);
    let orch: Arc<SyncOrchestrator> = store.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orch.refresh_all().await.unwrap();
    store
}

#[tokio::test]
async fn board_roadmap_serves_flat_priority_cards_carrying_the_enriched_state() {
    let store = seeded_store(vec![
        seed_outcome(710, "Later thing", &["outcome", "horizon.later"]),
        seed_outcome(700, "Now thing", &["outcome", "horizon.now"]),
    ])
    .await;

    let cards = board_roadmap(&store.select());

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
    let store = seeded_store(vec![
        (outcome, outcome_labels),
        (iteration, iteration_labels),
    ])
    .await;

    let iterations = board_iterations(&store.select());
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

#[test]
fn sort_ready_by_membership_orders_the_ready_bucket_and_leaves_other_buckets_untouched() {
    let stories = vec![
        serde_json::json!({"id": 9002, "state": "icebox"}),
        serde_json::json!({"id": 5050, "state": "ready"}),
        serde_json::json!({"id": 9001, "state": "icebox"}),
        serde_json::json!({"id": 5001, "state": "ready"}),
        serde_json::json!({"id": 5002, "state": "ready"}),
    ];
    let current_order = [5002, 5001];

    let sorted = sort_ready_by_membership(stories, &current_order);

    let ids: Vec<u64> = sorted.iter().filter_map(|s| s["id"].as_u64()).collect();
    assert_eq!(
        ids,
        vec![9002, 9001, 5002, 5001, 5050],
        "buckets keep their order of first appearance (icebox before ready); the icebox bucket keeps its original relative order; the ready bucket sorts members first by membership position, then absentees by issue number"
    );
}
