#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

async fn build_orchestrator() -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let story = Story::new_minimal(
        StoryId::new(200),
        "t".into(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![story]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

fn label_args(label: &str, operation: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(200));
    args.insert("label".into(), serde_json::json!(label));
    args.insert("operation".into(), serde_json::json!(operation));
    args
}

#[tokio::test]
async fn label_story_rejects_iteration_label_with_tool_retired_kind_1402() {
    let orch = build_orchestrator().await;
    let result = handle_label_story(
        &label_args("iteration-Telescopium.100", "add"),
        "pm:claude-code",
        Some(&orch),
    )
    .await;
    let err = result.expect_err("iteration-axis label-add must be rejected post-#1402 retire");
    assert!(
        err.contains("\"kind\":\"tool-retired\""),
        "rejection cites tool-retired kind: {err}"
    );
    assert!(
        err.contains("\"successor\":\"add_story_to_iteration\""),
        "rejection cites successor: {err}"
    );
    assert!(
        err.contains("\"retired_axis\":\"iteration-label\""),
        "rejection cites retired_axis: {err}"
    );
    assert!(
        err.contains("iteration-Telescopium.100"),
        "rejection echoes the rejected label for caller-clarity: {err}"
    );
}

#[tokio::test]
async fn label_story_rejects_iteration_label_for_different_names_1402() {
    let orch = build_orchestrator().await;
    for label in [
        "iteration-Pegasus.100",
        "iteration-Big-Dipper.200",
        "iteration-Cassiopeia.50",
        "iteration-Carina.999",
    ] {
        let result =
            handle_label_story(&label_args(label, "add"), "pm:claude-code", Some(&orch)).await;
        let err = result.expect_err(&format!("{label} must be rejected"));
        assert!(
            err.contains("tool-retired"),
            "{label}: rejection cites tool-retired kind: {err}"
        );
    }
}

#[tokio::test]
async fn label_story_accepts_non_iteration_labels_unchanged_1402() {
    let orch = build_orchestrator().await;
    for label in [
        "desk-check-fail",
        "deferred",
        "icebox",
        "ready",
        "delivered",
        "parent-epic-#1384",
        "awaiting team",
    ] {
        let result =
            handle_label_story(&label_args(label, "add"), "pm:claude-code", Some(&orch)).await;
        assert!(
            result.is_ok(),
            "{label} should NOT be affected by #1402 retire: {result:?}"
        );
    }
    let story = orch.story(200).expect("story present");
    assert!(
        story
            .tags
            .contains(&crate::product_management::story::Tag::DeskCheckFail),
        "the desk-check-fail label classifies into the typed tag at the tool edge; got {:?}",
        story.tags
    );
    orch.refresh_all().await.unwrap();
    let story = orch.story(200).expect("story present after refresh");
    assert_eq!(
        story.parent_epic().map(|e| e.value()),
        Some(1384),
        "generic labels reach the adapter edge and decode back typed on refresh"
    );
}

#[tokio::test]
async fn label_story_rejects_iteration_label_before_canonical_store_check_1402() {
    let result = handle_label_story(
        &label_args("iteration-Telescopium.100", "add"),
        "pm:claude-code",
        None,
    )
    .await;
    let err = result.expect_err("iteration-label rejection fires even without refresh-driver");
    assert!(
        err.contains("tool-retired"),
        "rejection comes from retire-gate not refresh-driver-required: {err}"
    );
}

#[tokio::test]
async fn label_story_rejects_malformed_label_shapes_through_normal_path_1402() {
    let orch = build_orchestrator().await;
    for label in [
        "iteration-",
        "iteration-Pegasus",
        "iteration-Pegasus.",
        "iteration-Pegasus.foo",
        "iteration-#1400",
    ] {
        let result =
            handle_label_story(&label_args(label, "add"), "pm:claude-code", Some(&orch)).await;
        assert!(
            result.is_ok() || !result.as_ref().unwrap_err().contains("tool-retired"),
            "{label} should NOT match the canonical iteration-label-shape (so it doesn't trip the retire-gate): {result:?}"
        );
    }
}

#[tokio::test]
async fn label_story_remove_iteration_label_still_works_for_migration_1402() {
    let orch = build_orchestrator().await;
    let result = handle_label_story(
        &label_args("iteration-Telescopium.100", "remove"),
        "pm:claude-code",
        Some(&orch),
    )
    .await;
    assert!(
        result.is_ok(),
        "remove operation must still work for cleanup of legacy iteration-labels: {result:?}"
    );
}

#[tokio::test]
async fn non_pm_role_rejected_before_retire_gate_fires_1402() {
    let orch = build_orchestrator().await;
    let result = handle_label_story(
        &label_args("iteration-Telescopium.100", "add"),
        "dev:claude-code",
        Some(&orch),
    )
    .await;
    let err = result.expect_err("non-PM rejected at role-gate");
    assert!(
        err.contains("restricted to pm") || err.contains("pm-lane"),
        "non-PM rejection comes from role-gate (precedes retire-gate): {err}"
    );
    assert!(
        !err.contains("tool-retired"),
        "non-PM error must NOT mention tool-retired (role-gate wins precedence): {err}"
    );
}
