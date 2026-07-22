#![allow(unused_imports)]

use super::super::tools_stories::*;
use chrono::{TimeZone, Utc};
use integrations::in_memory::InMemorySourceAdapter;
use product_management::iteration::{Iteration, IterationAction, IterationId};
use product_management::story::{Story, StoryId, Work};
use source_codec::mapper::decode_iteration_from_story;
use source_codec::relationships::{IterationRelationships, Relationships};
use std::sync::Arc;
use store::{AppAction, Dispatch};
use sync::SyncOrchestrator;
use tokio::sync::mpsc::UnboundedReceiver;

fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
    Dispatch::recording()
}

fn upserted_iterations(rx: &mut UnboundedReceiver<AppAction>) -> Vec<Iteration> {
    let mut found = Vec::new();
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Iteration(IterationAction::Upserted(iterations)) = action {
            found.extend(iterations);
        }
    }
    found
}

fn stories_of(iterations: &[Iteration], id: u64) -> Vec<StoryId> {
    iterations
        .iter()
        .rev()
        .find(|it| it.id == IterationId::new(id))
        .map(|it| it.stories.clone())
        .unwrap_or_else(|| panic!("iteration #{id} present in the optimistic dispatch"))
}

async fn build_orchestrator(
    seeds: Vec<(Story, Vec<String>)>,
) -> (
    store_test_support::SeededStore,
    Arc<SyncOrchestrator>,
    store::Select,
) {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let iterations: Vec<Iteration> = seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
        .map(|(story, labels)| decode_iteration_from_story(story, labels, false))
        .collect();
    let store = store_test_support::SeededStore::new(vec![], iterations, vec![]);
    let orch: Arc<SyncOrchestrator> = store.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orch.refresh_all().await.unwrap();
    let select = store.select();
    (store, orch, select)
}

fn iteration_seed(id: u64, name: &str) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration {name}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, vec!["iteration".into(), "ready".into()])
}

fn member_seed(id: u64, title: &str) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        title.into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, Vec::new())
}

fn args(
    iteration_id: u64,
    story_id: u64,
    position: u64,
) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("iteration_id".into(), serde_json::json!(iteration_id));
    m.insert("story_id".into(), serde_json::json!(story_id));
    m.insert("position".into(), serde_json::json!(position));
    m
}

#[tokio::test]
async fn pm_can_add_story_to_iteration_canonical_relation_recorded_1399() {
    let (_store, orch, select) = build_orchestrator(vec![
        iteration_seed(1397, "Telescopium"),
        member_seed(1500, "child story"),
    ])
    .await;
    let (dispatch, mut rx) = dispatch_channel();
    let result = handle_add_story_to_iteration(
        &args(1397, 1500, 100),
        "pm",
        Some(&orch),
        &dispatch,
        &select,
    )
    .await;
    assert!(
        result.is_ok(),
        "PM add_story_to_iteration must succeed: {result:?}"
    );
    assert_eq!(
        stories_of(&upserted_iterations(&mut rx), 1397),
        vec![StoryId::new(1500)],
        "the member is recorded in the iteration's ordered stories (parent-holds-order)"
    );
}

#[tokio::test]
async fn non_pm_non_owner_rejected_with_workflow_role_not_allowed_1399() {
    let (_store, orch, select) = build_orchestrator(vec![
        iteration_seed(1397, "Telescopium"),
        member_seed(1500, "x"),
    ])
    .await;
    for caller in [
        "designer:claude-code",
        "dev:claude-code",
        "intern:claude-code",
    ] {
        let (dispatch, _rx) = dispatch_channel();
        let result = handle_add_story_to_iteration(
            &args(1397, 1500, 100),
            role_from_name(caller),
            Some(&orch),
            &dispatch,
            &select,
        )
        .await;
        let err = result.expect_err(&format!("{caller} must be substrate-refused"));
        assert!(
            err.contains("workflow-role-not-allowed"),
            "{caller}: error must carry canonical kind=workflow-role-not-allowed; got {err}"
        );
        let role_short = caller.split(':').next().unwrap_or(caller);
        assert!(
            err.contains(&format!("\"actor_role\":\"{role_short}\"")),
            "{caller}: error must name firing role={role_short}; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"add-to-iteration\""),
            "{caller}: error must cite add-to-iteration transition; got {err}"
        );
        assert!(
            err.contains("\"allowed_roles\":[\"pm\"]"),
            "{caller}: error must enumerate allowed-roles=[pm]; got {err}"
        );
    }
}

#[tokio::test]
async fn owner_bypasses_add_story_to_iteration_via_superset_1548() {
    let (_store, orch, select) = build_orchestrator(vec![
        iteration_seed(1397, "Telescopium"),
        member_seed(1500, "child story"),
    ])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let result = handle_add_story_to_iteration(
        &args(1397, 1500, 100),
        "owner",
        Some(&orch),
        &dispatch,
        &select,
    )
    .await;
    assert!(
        result.is_ok(),
        "owner (owner) bypasses the pm-only add-to-iteration role-guard via the #1548 superset: {result:?}"
    );
}

#[tokio::test]
async fn parent_must_be_iteration_storytype_rejects_non_iteration_1399() {
    let (_store, orch, select) = build_orchestrator(vec![
        member_seed(2000, "not an iteration"),
        member_seed(1500, "child"),
    ])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let result = handle_add_story_to_iteration(
        &args(2000, 1500, 100),
        "pm",
        Some(&orch),
        &dispatch,
        &select,
    )
    .await;
    let err = result.expect_err("non-Iteration parent must be rejected");
    assert!(
        err.contains("not an Iteration"),
        "rejection cites story_type mismatch: {err}"
    );
}

#[tokio::test]
async fn self_add_rejected_1399() {
    let (_store, orch, select) =
        build_orchestrator(vec![iteration_seed(1397, "Telescopium")]).await;
    let (dispatch, _rx) = dispatch_channel();
    let result = handle_add_story_to_iteration(
        &args(1397, 1397, 100),
        "pm",
        Some(&orch),
        &dispatch,
        &select,
    )
    .await;
    let err = result.expect_err("self-add must be rejected");
    assert!(
        err.contains("cannot be added to itself"),
        "self-add error cites the canonical rejection reason: {err}"
    );
}

#[tokio::test]
async fn missing_iteration_id_rejected_1399() {
    let (dispatch, _rx) = dispatch_channel();
    let select = store::Select::new(std::sync::Weak::new());
    let mut m = serde_json::Map::new();
    m.insert("story_id".into(), serde_json::json!(1500));
    m.insert("position".into(), serde_json::json!(100));
    let result = handle_add_story_to_iteration(&m, "pm", None, &dispatch, &select).await;
    let err = result.expect_err("missing iteration_id must be rejected");
    assert!(
        err.contains("iteration_id"),
        "rejection names the missing arg: {err}"
    );
}

#[tokio::test]
async fn missing_story_id_rejected_1399() {
    let (dispatch, _rx) = dispatch_channel();
    let select = store::Select::new(std::sync::Weak::new());
    let mut m = serde_json::Map::new();
    m.insert("iteration_id".into(), serde_json::json!(1397));
    m.insert("position".into(), serde_json::json!(100));
    let result = handle_add_story_to_iteration(&m, "pm", None, &dispatch, &select).await;
    let err = result.expect_err("missing story_id must be rejected");
    assert!(
        err.contains("story_id"),
        "rejection names the missing arg: {err}"
    );
}

#[tokio::test]
async fn missing_position_rejected_1399() {
    let (dispatch, _rx) = dispatch_channel();
    let select = store::Select::new(std::sync::Weak::new());
    let mut m = serde_json::Map::new();
    m.insert("iteration_id".into(), serde_json::json!(1397));
    m.insert("story_id".into(), serde_json::json!(1500));
    let result = handle_add_story_to_iteration(&m, "pm", None, &dispatch, &select).await;
    let err = result.expect_err("missing position must be rejected");
    assert!(
        err.contains("position"),
        "rejection names the missing arg: {err}"
    );
}

#[tokio::test]
async fn add_records_the_member_in_the_iterations_ordered_stories_1399() {
    let (_store, orch, select) = build_orchestrator(vec![
        iteration_seed(1397, "Telescopium"),
        member_seed(1500, "child"),
    ])
    .await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_add_story_to_iteration(
        &args(1397, 1500, 250),
        "pm",
        Some(&orch),
        &dispatch,
        &select,
    )
    .await
    .unwrap();
    assert_eq!(
        stories_of(&upserted_iterations(&mut rx), 1397),
        vec![StoryId::new(1500)],
        "the member is recorded in the iteration's ordered stories (the parent holds the order)"
    );
}

#[tokio::test]
async fn replace_semantic_overwrites_existing_membership_1399() {
    let (mut it1, it1_labels) = iteration_seed(1397, "Telescopium");
    it1.body = Some(
        IterationRelationships {
            outcome: None,
            stories: vec![StoryId::new(1500)],
        }
        .write_into_body("## composed"),
    );
    let it2 = iteration_seed(1396, "Sextans");
    let member = member_seed(1500, "moving target");
    let (_store, orch, select) = build_orchestrator(vec![(it1, it1_labels), it2, member]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_add_story_to_iteration(
        &args(1396, 1500, 300),
        "pm",
        Some(&orch),
        &dispatch,
        &select,
    )
    .await
    .unwrap();
    let dispatched = upserted_iterations(&mut rx);
    assert_eq!(
        stories_of(&dispatched, 1396),
        vec![StoryId::new(1500)],
        "the member is recorded in the new iteration"
    );
    assert!(
        stories_of(&dispatched, 1397).is_empty(),
        "single-iteration-membership invariant: the story is cleared from the prior iteration"
    );
}
