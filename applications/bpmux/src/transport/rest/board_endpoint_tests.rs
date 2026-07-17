#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::board::{board_iterations, board_outcomes, ordered_board_stories};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use product_management::iteration::{Iteration, IterationId};
use product_management::outcome::OutcomeId;
use product_management::story::{Story, StoryId, Work};
use source_codec::mapper::{decode_iteration_from_story, decode_outcome_from_story};
use source_codec::relationships::{IterationRelationships, OutcomeRelationships, Relationships};
use std::sync::Arc;

async fn seeded_store(seeds: Vec<(Story, Vec<String>)>) -> crate::store::SeededStore {
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
    let store = crate::store::SeededStore::new(seeded, iterations, vec![]);
    let orch: Arc<SyncOrchestrator> = store.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orch.refresh_all().await.unwrap();
    store
}

fn iteration(id: u64) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        "Iteration Vela".to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.body = Some("## Iteration overview\n\ncomposed".to_string());
    (
        s,
        vec![
            "iteration".into(),
            "ready".into(),
            "iteration-review-closed.designer".into(),
            "iteration-dev-ok.dev".into(),
        ],
    )
}

fn member(id: u64, iteration_id: u64, position: u32) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("member #{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (
        s,
        vec![format!("parent-iteration-#{iteration_id}.{position}")],
    )
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

fn outcome(id: u64, iterations: Vec<u64>) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        "An outcome".to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: iterations.into_iter().map(IterationId::new).collect(),
        }
        .write_into_body("## Outcome\n"),
    );
    (s, vec!["outcome".into(), "horizon.now".into()])
}

fn iteration_with_stories(id: u64, stories: Vec<u64>) -> (Story, Vec<String>) {
    let (mut s, labels) = iteration(id);
    s.body = Some(
        IterationRelationships {
            outcome: None,
            stories: stories.into_iter().map(StoryId::new).collect(),
        }
        .write_into_body(s.body.as_deref().unwrap_or_default()),
    );
    (s, labels)
}

fn building_iteration(id: u64, outcome_id: u64, stories: Vec<u64>) -> (Story, Vec<String>) {
    let (mut s, labels) = iteration(id);
    s.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(outcome_id)),
            stories: stories.into_iter().map(StoryId::new).collect(),
        }
        .write_into_body(s.body.as_deref().unwrap_or_default()),
    );
    (s, labels)
}

fn plain_member(id: u64) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("member-{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 7, 7, 0, 0, 0).unwrap(),
        "pm".into(),
    )
}

fn flagged_iteration(id: u64, is_current: bool, complete: bool, stories: Vec<u64>) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 7, 7, 0, 0, 0).unwrap(),
        filed_by: "pm".into(),
        outcome: None,
        iteration_order: None,
        stories: stories.into_iter().map(StoryId::new).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current,
        complete,
    }
}

async fn board_ready_order(stories: Vec<Story>, iterations: Vec<Iteration>) -> Vec<u64> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let store = crate::store::SeededStore::new(vec![], iterations, vec![]);
    let refresh: Arc<SyncOrchestrator> = store.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    refresh.refresh_all().await.unwrap();
    ordered_board_stories(&store.select())
        .iter()
        .filter(|s| s["state"].as_str() == Some("ready"))
        .map(|s| s["id"].as_u64().expect("ready story id is numeric"))
        .collect()
}

#[tokio::test]
async fn a_completed_iteration_flagged_current_puts_no_member_in_the_board_ready_column() {
    let order = board_ready_order(
        vec![plain_member(5001)],
        vec![flagged_iteration(9000, true, true, vec![5001])],
    )
    .await;

    assert!(
        order.is_empty(),
        "a completed iteration's member is Icebox, never in the board ready column; got {order:?}"
    );
}

#[tokio::test]
async fn a_stale_completed_current_does_not_seed_the_board_ready_order() {
    let order = board_ready_order(
        vec![plain_member(5001), plain_member(5002)],
        vec![
            flagged_iteration(9000, true, true, vec![5001]),
            flagged_iteration(9001, true, false, vec![5002, 5001]),
        ],
    )
    .await;

    assert_eq!(
        order,
        vec![5002, 5001],
        "the open current iteration's membership order seeds the board ready order; the stale \
         completed-current is ignored; got {order:?}"
    );
}

#[tokio::test]
async fn an_optimistic_reject_is_visible_on_the_board_snapshot_before_it_reaches_github() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_labels(9100, vec!["bug".into()]);
    adapter.seed_stories(vec![Story::new_minimal(
        StoryId::new(9100),
        "[Bug] optimistic board visibility".to_string(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    )]);
    let current = Iteration {
        id: IterationId::new(9000),
        title: "Iteration Vela".to_string(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        filed_by: "pm".into(),
        outcome: None,
        iteration_order: None,
        stories: vec![StoryId::new(9100)],
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    };
    let store = crate::store::SeededStore::new(vec![], vec![current], vec![]);
    let refresh: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    refresh.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(9100));
    args.insert(
        "reason".into(),
        serde_json::json!("scope mismatch — rework"),
    );
    crate::bridge::tools_stories::handle_reject_story(
        &args,
        "pm:human",
        "pm",
        None,
        Some(&refresh),
        &select,
    )
    .await
    .expect("pm rejects the story");

    let stories = ordered_board_stories(&store.select());
    assert_eq!(
        find_story(&stories, 9100)["state"].as_str(),
        Some("ready"),
        "the board snapshot the renderer receives reflects the reject immediately — user-facing state moved acceptance -> ready"
    );
    assert!(
        adapter.pushed_events().is_empty(),
        "and it is visible OPTIMISTICALLY — the board shows the reject before anything reaches GitHub (no drain driven): {:?}",
        adapter.pushed_events()
    );
}

#[tokio::test]
async fn outcomes_delta_carries_rolled_up_readiness_and_building_iterations_1738() {
    let store = seeded_store(vec![
        outcome(3000, vec![3010]),
        building_iteration(3010, 3000, vec![3011]),
    ])
    .await;
    let outcomes = board_outcomes(&store.select());
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
    let store = seeded_store(vec![outcome(3000, vec![])]).await;
    let stories = ordered_board_stories(&store.select());
    assert!(
        !stories.iter().any(|s| s["id"].as_u64() == Some(3000)),
        "an outcome is not a story — it never appears in the board story collection"
    );
}

#[tokio::test]
async fn iteration_readiness_rides_the_iterations_wire_1703() {
    let store = seeded_store(vec![iteration_with_stories(2000, vec![2001])]).await;
    let iterations = board_iterations(&store.select());
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
    let store = seeded_store(vec![iteration(2000)]).await;
    let stories = ordered_board_stories(&store.select());
    assert!(
        !stories.iter().any(|s| s["id"].as_u64() == Some(2000)),
        "an iteration is a planning container, not a story — it never appears in the board story collection (readiness rides the iterations wire)"
    );
}

#[tokio::test]
async fn non_iteration_story_has_no_readiness_key_1703() {
    let store = seeded_store(vec![member(2001, 2000, 100)]).await;
    let stories = ordered_board_stories(&store.select());
    assert!(
        find_story(&stories, 2001).get("readiness").is_none(),
        "readiness is an iteration-only concern; feature stories carry no readiness key"
    );
}
