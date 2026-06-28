#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::board::{board_iterations, board_roadmap};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{Story, StoryState, StoryType};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn seed(number: u64, title: &str, story_type: StoryType, labels: &[&str]) -> Story {
    let mut s = Story::new_minimal(
        number,
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

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    orch
}

#[tokio::test]
async fn board_roadmap_serves_flat_priority_cards_carrying_the_enriched_state() {
    let refresh = build_refresh(vec![
        seed(
            710,
            "Later thing",
            StoryType::Outcome,
            &["outcome", "horizon.later"],
        ),
        seed(
            700,
            "Now thing",
            StoryType::Outcome,
            &["outcome", "horizon.now"],
        ),
    ])
    .await;

    let cards = board_roadmap(&refresh);

    assert_eq!(cards.len(), 2, "both non-retired outcomes reach the wire");
    assert_eq!(
        cards[0]["number"], 700,
        "the served layer is flat in priority order — Now precedes Later"
    );
    assert_eq!(cards[1]["number"], 710);
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
    let mut iteration = seed(810, "Iteration", StoryType::Iteration, &[]);
    iteration.body = Some(
        IterationRelationships {
            outcome: Some(700),
            stories: vec![],
        }
        .write_into_body("## composed"),
    );
    let mut outcome = seed(
        700,
        "Outcome",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    );
    outcome.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: vec![810],
        }
        .write_into_body("## Outcome\n"),
    );
    let refresh = build_refresh(vec![outcome, iteration]).await;

    let iterations = board_iterations(&refresh);
    let mapped = iterations
        .iter()
        .find(|i| i["number"] == 810)
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
