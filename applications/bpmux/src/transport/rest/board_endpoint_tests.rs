#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::board::{board_iterations, board_outcomes, ordered_board_stories};
use crate::product_management::board::adapters::github::{
    decode_iteration_from_story, decode_outcome_from_story,
};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::product_management::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::outcome::OutcomeId;
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::store::Select;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

async fn orchestrator(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
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
    let iterations: Vec<Iteration> = stories
        .iter()
        .filter(|s| {
            s.raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case("iteration"))
        })
        .map(decode_iteration_from_story)
        .collect();
    orch.bind_select(Select::seeded_for_board(
        &orch.store(),
        seeded,
        iterations,
        vec![],
    ));
    orch
}

fn iteration(id: u64) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        "Iteration Vela".to_string(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.body = Some("## Iteration overview\n\ncomposed".to_string());
    s.raw_labels = vec![
        "iteration".into(),
        "ready".into(),
        "iteration-review-closed.designer".into(),
        "iteration-dev-ok.dev".into(),
    ];
    s
}

fn member(id: u64, iteration_id: u64, position: u32) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("member #{id}"),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec![format!("parent-iteration-#{iteration_id}.{position}")];
    s
}

fn find(stories: &[serde_json::Value], id: u64) -> &serde_json::Value {
    stories
        .iter()
        .find(|s| s["id"].as_u64() == Some(id))
        .expect("story present in snapshot")
}

fn find_story(stories: &[serde_json::Value], id: u64) -> &serde_json::Value {
    stories
        .iter()
        .find(|s| s["id"].as_u64() == Some(id))
        .expect("story present in snapshot")
}

fn outcome(id: u64, iterations: Vec<u64>) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        "An outcome".to_string(),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["outcome".into(), "horizon.now".into()];
    s.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: iterations.into_iter().map(IterationId::new).collect(),
        }
        .write_into_body("## Outcome\n"),
    );
    s
}

fn iteration_with_stories(id: u64, stories: Vec<u64>) -> Story {
    let mut s = iteration(id);
    s.body = Some(
        IterationRelationships {
            outcome: None,
            stories: stories.into_iter().map(StoryId::new).collect(),
        }
        .write_into_body(s.body.as_deref().unwrap_or_default()),
    );
    s
}

fn building_iteration(id: u64, outcome_id: u64, stories: Vec<u64>) -> Story {
    let mut s = iteration(id);
    s.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(outcome_id)),
            stories: stories.into_iter().map(StoryId::new).collect(),
        }
        .write_into_body(s.body.as_deref().unwrap_or_default()),
    );
    s
}

#[tokio::test]
async fn outcomes_delta_carries_rolled_up_readiness_and_building_iterations_1738() {
    let orch = orchestrator(vec![
        outcome(3000, vec![3010]),
        building_iteration(3010, 3000, vec![3011]),
    ])
    .await;
    let outcomes = board_outcomes(&orch);
    let outcome = find(&outcomes, 3000);
    let rolled = &outcome["outcome_readiness"];
    assert_eq!(
        rolled["iteration_count"].as_u64(),
        Some(1),
        "one iteration builds the outcome"
    );
    assert_eq!(rolled["gap_count"].as_u64(), Some(0));
    assert_eq!(
        rolled["all_ready"].as_bool(),
        Some(true),
        "the building iteration is fully ready → the outcome's readiness rolls up to ready, exposed for #1704/#1737"
    );
    let building = outcome["iterations"]
        .as_array()
        .expect("the outcome carries its building iterations");
    assert_eq!(building.len(), 1, "the one building iteration is present");
    assert_eq!(building[0]["id"].as_u64(), Some(3010));
    assert_eq!(building[0]["position"].as_u64(), Some(1));
    assert_eq!(
        building[0]["readiness"]["review_closed"].as_bool(),
        Some(true),
        "each building iteration carries its own readiness for the drill-in (#1737)"
    );
}

#[tokio::test]
async fn outcomes_never_ride_in_the_board_story_collection_1738() {
    let orch = orchestrator(vec![outcome(3000, vec![])]).await;
    let stories = ordered_board_stories(&orch);
    assert!(
        !stories.iter().any(|s| s["id"].as_u64() == Some(3000)),
        "an outcome is not a story — it never appears in the board story collection"
    );
}

#[tokio::test]
async fn iteration_readiness_rides_the_iterations_wire_1703() {
    let orch = orchestrator(vec![iteration_with_stories(2000, vec![2001])]).await;
    let iterations = board_iterations(&orch);
    let readiness = &find(&iterations, 2000)["readiness"];
    assert_eq!(
        readiness["review_closed"].as_bool(),
        Some(true),
        "the designer review-close derives review_closed=true on the iterations wire, exposed for #1704 to consume"
    );
    assert_eq!(readiness["drafted"].as_bool(), Some(true));
    assert_eq!(readiness["stories_ordered"].as_bool(), Some(true));
    assert_eq!(
        readiness["dev_ok"].as_bool(),
        Some(true),
        "the dev-OK derives dev_ok=true on the iterations wire, exposed for #1704 to consume"
    );
}

#[tokio::test]
async fn iterations_never_ride_in_the_board_story_collection_1703() {
    let orch = orchestrator(vec![iteration(2000)]).await;
    let stories = ordered_board_stories(&orch);
    assert!(
        !stories.iter().any(|s| s["id"].as_u64() == Some(2000)),
        "an iteration is a planning container, not a story — it never appears in the board story collection (readiness rides the iterations wire)"
    );
}

#[tokio::test]
async fn non_iteration_story_has_no_readiness_key_1703() {
    let orch = orchestrator(vec![member(2001, 2000, 100)]).await;
    let stories = ordered_board_stories(&orch);
    assert!(
        find_story(&stories, 2001).get("readiness").is_none(),
        "readiness is an iteration-only concern; feature stories carry no readiness key"
    );
}
