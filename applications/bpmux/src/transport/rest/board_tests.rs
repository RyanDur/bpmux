#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::board::{board_iterations, board_roadmap};
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
use crate::product_management::outcome::OutcomeId;
use crate::product_management::story::{BoardState, Story, StoryId};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn iteration_seed(id: u64, title: &str) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["iteration".to_string()];
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

struct SeededOutcomes(Vec<crate::product_management::outcome::Outcome>);
impl OutcomeReader for SeededOutcomes {
    fn outcomes(&self) -> Vec<crate::product_management::outcome::Outcome> {
        self.0.clone()
    }
}

struct SeededIterations(Vec<Iteration>);
impl IterationReader for SeededIterations {
    fn iterations(&self) -> Vec<Iteration> {
        self.0.clone()
    }
}

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories.clone());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    let seeded: Vec<_> = stories
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
    let mut iteration = iteration_seed(810, "Iteration");
    iteration.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(700)),
            stories: vec![],
        }
        .write_into_body("## composed"),
    );
    let mut outcome = seed_outcome(700, "Outcome", &["outcome", "horizon.now"]);
    outcome.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: vec![IterationId::new(810)],
        }
        .write_into_body("## Outcome\n"),
    );
    let refresh = build_refresh(vec![outcome, iteration]).await;

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
