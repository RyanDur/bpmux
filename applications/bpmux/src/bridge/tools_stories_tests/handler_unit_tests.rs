#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::board::domain::StoryType;
use crate::session::velocity::classify_story;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn accept_requires_refresh_after_5n_fallback_retirement() {
    let args = serde_json::json!({"number": 213, "would_file_bug": false});
    let result = handle_accept_story(args.as_object().unwrap(), "pm:claude-code", None, None).await;
    assert!(
        result.is_err(),
        "expected refresh-required Err, got {result:?}"
    );
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn accept_rejects_unknown_role_via_handler() {
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let args = serde_json::json!({"number": 1, "would_file_bug": false});
    let result = handle_accept_story(
        args.as_object().unwrap(),
        "intern:claude-code",
        Some(&orchestrator),
        None,
    )
    .await;
    let err = result.expect_err("intern role should be substrate-refused");
    assert!(
        err.contains("workflow-role-not-allowed"),
        "structured kind cites workflow-role-not-allowed; got {err}"
    );
    assert!(
        err.contains("\"actor_role\":\"intern\""),
        "structured error names firing role; got {err}"
    );
}

#[tokio::test]
async fn accept_requires_would_file_bug_arg() {
    let args = serde_json::json!({"number": 213});
    let err = handle_accept_story(args.as_object().unwrap(), "pm:claude-code", None, None)
        .await
        .expect_err("missing would_file_bug should be rejected");
    assert!(
        err.contains("would_file_bug"),
        "rejection cites the missing arg: {err}"
    );
    assert!(
        err.contains("file a bug"),
        "rejection invokes canonical bug-question phrasing: {err}"
    );
}

#[tokio::test]
async fn accept_blocks_when_would_file_bug_true_without_force() {
    let args = serde_json::json!({"number": 213, "would_file_bug": true});
    let err = handle_accept_story(args.as_object().unwrap(), "pm:claude-code", None, None)
        .await
        .expect_err("would_file_bug=true should block accept");
    assert!(
        err.contains("reject_story"),
        "block routes to alternate gate: {err}"
    );
    assert!(
        err.contains("force=true") && err.contains("bypass_reason"),
        "block surfaces the bypass affordance: {err}"
    );
}

#[tokio::test]
async fn accept_force_bypass_requires_bypass_reason() {
    let args = serde_json::json!({"number": 213, "would_file_bug": true, "force": true});
    let err = handle_accept_story(args.as_object().unwrap(), "pm:claude-code", None, None)
        .await
        .expect_err("force without bypass_reason should be rejected");
    assert!(
        err.contains("bypass_reason"),
        "rejection cites the missing arg: {err}"
    );
}

#[tokio::test]
async fn pass_desk_check_requires_would_file_bug_arg() {
    let args = serde_json::json!({"number": 201, "acceptor": "owner"});
    let err = handle_pass_desk_check(
        args.as_object().unwrap(),
        "designer:claude-code",
        None,
        None,
    )
    .await
    .expect_err("missing would_file_bug should be rejected");
    assert!(
        err.contains("would_file_bug"),
        "rejection cites the missing arg: {err}"
    );
    assert!(
        err.contains("file a bug"),
        "rejection invokes canonical bug-question phrasing: {err}"
    );
}

#[tokio::test]
async fn pass_desk_check_blocks_when_would_file_bug_true_without_force() {
    let args = serde_json::json!({"number": 201, "acceptor": "owner", "would_file_bug": true});
    let err = handle_pass_desk_check(
        args.as_object().unwrap(),
        "designer:claude-code",
        None,
        None,
    )
    .await
    .expect_err("would_file_bug=true should block pass");
    assert!(
        err.contains("fail_desk_check"),
        "block routes to alternate gate: {err}"
    );
}

#[tokio::test]
async fn accept_force_bypass_with_reason_records_in_gate_payload() {
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use crate::board::{BoardEvent, BoardStore, RefreshDriver, SyncOrchestrator};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![Story::new_minimal(
        777,
        "[Bug] bypass-target".to_string(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let args = serde_json::json!({
        "number": 777,
        "would_file_bug": true,
        "force": true,
        "bypass_reason": "test-data churn; not feature-bug class"
    });
    let result = handle_accept_story(
        args.as_object().unwrap(),
        "ryan:human",
        Some(&orchestrator),
        None,
    )
    .await;
    assert!(result.is_ok(), "force+reason should succeed: {result:?}");

    let pushed = adapter.pushed_events();
    let gate_event = pushed
        .iter()
        .find_map(|e| match e {
            BoardEvent::GateCommentAppended {
                number, payload, ..
            } if *number == 777 => Some(payload.clone()),
            _ => None,
        })
        .expect("gate-comment event recorded");
    let recorded = gate_event.expect("payload set on bypass");
    assert!(
        recorded.contains("bug-question bypass") && recorded.contains("test-data churn"),
        "bypass reason recorded in gate-comment payload: {recorded}"
    );
}

#[tokio::test]
async fn reject_allows_pm_designer_dev_owner() {
    for caller in [
        "ryan:human",
        "pm:claude-code",
        "designer:claude-code",
        "dev:claude-code",
    ] {
        let args = serde_json::json!({"number": 1, "reason": "needs rework"});
        let result = handle_reject_story(args.as_object().unwrap(), caller, None).await;
        assert!(
            result.is_ok() || !result.as_ref().unwrap_err().contains("restricted to"),
            "reject must allow {caller}; got: {result:?}"
        );
    }
}

#[tokio::test]
async fn reject_rejects_unknown_role_via_handler() {
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let args = serde_json::json!({"number": 1, "reason": "test"});
    let result = handle_reject_story(
        args.as_object().unwrap(),
        "intern:claude-code",
        Some(&orchestrator),
    )
    .await;
    let err = result.expect_err("intern role should be substrate-refused");
    assert!(
        err.contains("workflow-role-not-allowed"),
        "structured kind cites workflow-role-not-allowed; got {err}"
    );
    assert!(
        err.contains("\"actor_role\":\"intern\""),
        "structured error names firing role; got {err}"
    );
}
