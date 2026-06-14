#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::board::BoardEvent;
use crate::board::BoardStore;
use crate::board::RefreshDriver;
use crate::board::SyncOrchestrator;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState, StoryType};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

async fn build_orchestrator(stories: Vec<Story>) -> (Arc<BoardStore>, Arc<dyn RefreshDriver>) {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    (store, orch)
}

fn iteration_seed(number: u64, theme: &str) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration {theme}"),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = StoryType::Iteration;
    s.raw_labels = vec!["iteration".into(), "ready".into()];
    s
}

fn member_seed(number: u64, title: &str) -> Story {
    Story::new_minimal(
        number,
        title.into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    )
}

fn args(
    iteration_number: u64,
    story_number: u64,
    position: u64,
) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert(
        "iteration_number".into(),
        serde_json::json!(iteration_number),
    );
    m.insert("story_number".into(), serde_json::json!(story_number));
    m.insert("position".into(), serde_json::json!(position));
    m
}

#[tokio::test]
async fn pm_can_add_story_to_iteration_canonical_relation_recorded_1399() {
    let (_store, orch) = build_orchestrator(vec![
        iteration_seed(1397, "Telescopium"),
        member_seed(1500, "child story"),
    ])
    .await;
    let result =
        handle_add_story_to_iteration(&args(1397, 1500, 100), "pm:claude-code", Some(&orch)).await;
    assert!(
        result.is_ok(),
        "PM add_story_to_iteration must succeed: {result:?}"
    );
    let drained = orch.drain_pending().await.unwrap();
    assert_eq!(drained, 1, "single canonical-relation enqueue");
}

#[tokio::test]
async fn non_pm_non_owner_rejected_with_workflow_role_not_allowed_1399() {
    let (_store, orch) = build_orchestrator(vec![
        iteration_seed(1397, "Telescopium"),
        member_seed(1500, "x"),
    ])
    .await;
    for caller in [
        "designer:claude-code",
        "dev:claude-code",
        "intern:claude-code",
    ] {
        let result =
            handle_add_story_to_iteration(&args(1397, 1500, 100), caller, Some(&orch)).await;
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
    let (_store, orch) = build_orchestrator(vec![
        iteration_seed(1397, "Telescopium"),
        member_seed(1500, "child story"),
    ])
    .await;
    let result =
        handle_add_story_to_iteration(&args(1397, 1500, 100), "ryan:human", Some(&orch)).await;
    assert!(
        result.is_ok(),
        "owner (ryan) bypasses the pm-only add-to-iteration role-guard via the #1548 superset: {result:?}"
    );
}

#[tokio::test]
async fn parent_must_be_iteration_storytype_rejects_non_iteration_1399() {
    let (_store, orch) = build_orchestrator(vec![
        member_seed(2000, "not an iteration"),
        member_seed(1500, "child"),
    ])
    .await;
    let result =
        handle_add_story_to_iteration(&args(2000, 1500, 100), "pm:claude-code", Some(&orch)).await;
    let err = result.expect_err("non-Iteration parent must be rejected");
    assert!(
        err.contains("not an Iteration"),
        "rejection cites story_type mismatch: {err}"
    );
}

#[tokio::test]
async fn self_add_rejected_1399() {
    let (_store, orch) = build_orchestrator(vec![iteration_seed(1397, "Telescopium")]).await;
    let result =
        handle_add_story_to_iteration(&args(1397, 1397, 100), "pm:claude-code", Some(&orch)).await;
    let err = result.expect_err("self-add must be rejected");
    assert!(
        err.contains("cannot be added to itself"),
        "self-add error cites the canonical rejection reason: {err}"
    );
}

#[tokio::test]
async fn missing_iteration_number_rejected_1399() {
    let mut m = serde_json::Map::new();
    m.insert("story_number".into(), serde_json::json!(1500));
    m.insert("position".into(), serde_json::json!(100));
    let result = handle_add_story_to_iteration(&m, "pm:claude-code", None).await;
    let err = result.expect_err("missing iteration_number must be rejected");
    assert!(
        err.contains("iteration_number"),
        "rejection names the missing arg: {err}"
    );
}

#[tokio::test]
async fn missing_story_number_rejected_1399() {
    let mut m = serde_json::Map::new();
    m.insert("iteration_number".into(), serde_json::json!(1397));
    m.insert("position".into(), serde_json::json!(100));
    let result = handle_add_story_to_iteration(&m, "pm:claude-code", None).await;
    let err = result.expect_err("missing story_number must be rejected");
    assert!(
        err.contains("story_number"),
        "rejection names the missing arg: {err}"
    );
}

#[tokio::test]
async fn missing_position_rejected_1399() {
    let mut m = serde_json::Map::new();
    m.insert("iteration_number".into(), serde_json::json!(1397));
    m.insert("story_number".into(), serde_json::json!(1500));
    let result = handle_add_story_to_iteration(&m, "pm:claude-code", None).await;
    let err = result.expect_err("missing position must be rejected");
    assert!(
        err.contains("position"),
        "rejection names the missing arg: {err}"
    );
}

#[tokio::test]
async fn enqueues_iteration_membership_set_event_with_correct_args_1399() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let iteration = iteration_seed(1397, "Telescopium");
    let member = member_seed(1500, "child");
    adapter.seed_stories(vec![iteration, member]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    handle_add_story_to_iteration(&args(1397, 1500, 250), "pm:claude-code", Some(&orch))
        .await
        .unwrap();
    let drained = orch.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(
        matches!(
            &pushed[0],
            BoardEvent::IterationMembershipSet {
                number: 1500,
                iteration_number: 1397,
                position: 250,
                ..
            }
        ),
        "pushed event must carry the right (number, iteration_number, position) triple; got {:?}",
        pushed[0]
    );
    let updated = store.story(1500).expect("member story present");
    assert_eq!(
        updated.iteration_membership,
        Some((1397, 250)),
        "typed field populated post-mutation"
    );
}

#[tokio::test]
async fn replace_semantic_overwrites_existing_membership_1399() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let it1 = iteration_seed(1397, "Telescopium");
    let it2 = iteration_seed(1396, "Sextans");
    let mut member = member_seed(1500, "moving target");
    member.iteration_membership = Some((1397, 100));
    member.raw_labels = vec!["ready".into(), "parent-iteration-#1397.100".into()];
    adapter.seed_stories(vec![it1, it2, member]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    handle_add_story_to_iteration(&args(1396, 1500, 300), "pm:claude-code", Some(&orch))
        .await
        .unwrap();
    let updated = store.story(1500).expect("member present after move");
    assert_eq!(
        updated.iteration_membership,
        Some((1396, 300)),
        "single-iteration-membership invariant: prior (1397, 100) replaced by (1396, 300)"
    );
}
