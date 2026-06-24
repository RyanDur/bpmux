#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::board::domain::StoryType;
use crate::transport::protocol::ClientMessage;
use crate::velocity::classify_story;

#[test]
fn role_from_name_extracts_short_role() {
    assert_eq!(role_from_name("dev:claude-code"), "dev");
    assert_eq!(role_from_name("pm"), "pm");
    assert_eq!(role_from_name("designer:cursor"), "designer");
    assert_eq!(role_from_name("owner:human"), "owner");
}

#[test]
fn require_role_allows_matching_role() {
    assert!(require_role("dev:claude-code", &["dev"], "start").is_ok());
    assert!(require_role("pm", &["pm"], "label").is_ok());
    assert!(require_role("owner:human", &["owner"], "accept").is_ok());
}

#[test]
fn require_role_owner_superset_bypasses_any_gate_yet_non_owner_stays_gated() {
    assert!(require_role("owner:human", &["pm"], "label").is_ok());
    assert!(require_role("owner:human", &["designer"], "fail desk-check").is_ok());
    assert!(require_role("dev:claude-code", &["pm"], "label").is_err());
}

#[test]
fn require_role_rejects_non_matching_role() {
    let result = require_role("designer:claude-code", &["dev"], "start");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to dev"));
}

#[test]
fn require_role_rejects_pm_from_dev_operations() {
    let result = require_role("pm:claude-code", &["dev"], "deliver");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to dev"));
}

#[test]
fn require_role_rejects_dev_from_pm_operations() {
    let result = require_role("dev:claude-code", &["pm"], "label");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[test]
fn require_role_rejects_agents_from_owner_only_operations() {
    let result = require_role("dev:claude-code", &["owner"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to owner"));

    let result = require_role("pm:claude-code", &["owner"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to owner"));
}

#[test]
fn require_role_allows_designer_for_desk_check_operations() {
    assert!(require_role("designer:claude-code", &["designer"], "pass desk-check").is_ok());
    assert!(require_role("designer", &["designer"], "fail desk-check").is_ok());
}

#[test]
fn require_role_rejects_dev_from_close_iteration() {
    let result = require_role("dev:claude-code", &["pm"], "close iteration");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[test]
fn require_role_rejects_designer_from_close_iteration() {
    let result = require_role("designer:claude-code", &["pm"], "close iteration");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[test]
fn require_role_lets_owner_bypass_close_iteration_via_the_1548_superset() {
    assert!(require_role("owner:human", &["pm"], "close iteration").is_ok());
}

#[test]
fn build_iteration_close_signal_broadcasts_to_all() {
    let signal = build_iteration_close_signal();
    let ClientMessage::Send {
        to,
        addressee,
        text,
        message_type,
    } = signal
    else {
        panic!("expected Send variant");
    };
    assert!(to.is_none(), "to should be None for full broadcast");
    assert_eq!(
        addressee.as_deref(),
        Some("all"),
        "addressee must be 'all' so every agent treats it as their turn"
    );
    assert!(
        text.contains("[iteration-close]"),
        "text must carry the [iteration-close] marker so agents can recognize the signal: got {text}"
    );
    assert!(
        text.contains("flush unsaved findings"),
        "text must name the externalize action: got {text}"
    );
    assert!(message_type.is_none());
}

#[test]
fn close_story_is_pm_only_gate_for_non_owner_roles() {
    for caller in ["dev:claude-code", "designer:claude-code"] {
        let result = require_role(caller, &["pm"], "close");
        assert!(
            result.is_err(),
            "{caller} (non-pm, non-owner) should not pass close_story permission gate"
        );
        assert!(result.unwrap_err().contains("restricted to pm"));
    }
    assert!(
        require_role("owner:human", &["pm"], "close").is_ok(),
        "the owner bypasses the pm-only close gate via the #1548 superset"
    );
}

#[test]
fn close_story_allows_pm() {
    assert!(require_role("pm:claude-code", &["pm"], "close").is_ok());
}

#[tokio::test]
async fn comment_on_story_rejects_empty_body() {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(42));
    args.insert("body".to_string(), serde_json::json!(""));
    let result = handle_comment_on_story(&args, "dev:claude-code", None).await;
    assert!(
        result.is_err(),
        "empty body must be rejected: got {result:?}"
    );
    assert!(result.unwrap_err().contains("must not be empty"));
}

#[tokio::test]
async fn comment_on_story_rejects_whitespace_only_body() {
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(42));
    args.insert("body".to_string(), serde_json::json!("   \n  "));
    let result = handle_comment_on_story(&args, "dev:claude-code", None).await;
    assert!(result.is_err(), "whitespace-only body must be rejected");
}

#[tokio::test]
async fn comment_on_story_rejects_when_refresh_absent() {
    for caller in ["dev:claude-code", "pm:claude-code", "designer:claude-code"] {
        let mut args = serde_json::Map::new();
        args.insert("number".to_string(), serde_json::json!(42));
        args.insert("body".to_string(), serde_json::json!("a real comment"));
        let result = handle_comment_on_story(&args, caller, None).await;
        assert!(
            result.is_err(),
            "caller={caller} expected Err, got {result:?}"
        );
        assert!(
            result.unwrap_err().contains("requires refresh"),
            "expected refresh-required error for caller={caller}"
        );
    }
}

#[test]
fn require_role_rejects_non_designer_non_owner_from_desk_check_operations() {
    for caller in ["dev:claude-code", "pm:claude-code"] {
        let result = require_role(caller, &["designer"], "pass desk-check");
        assert!(
            result.is_err(),
            "{caller} should not pass desk-check; got: {result:?}"
        );
        assert!(result.unwrap_err().contains("restricted to designer"));
    }
    assert!(
        require_role("owner:human", &["designer"], "pass desk-check").is_ok(),
        "the owner bypasses the designer-only require_role gate via the #1548 superset"
    );
}

fn reorder_args(
    number: u64,
    position: u64,
    state: &str,
) -> serde_json::Map<String, serde_json::Value> {
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(number));
    args.insert("position".into(), serde_json::json!(position));
    args.insert("state".into(), serde_json::json!(state));
    args
}

fn move_args(
    number: u64,
    target_state: &str,
    position: Option<u64>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(number));
    args.insert("target_state".into(), serde_json::json!(target_state));
    args.insert(
        "rationale".into(),
        serde_json::json!("test: move role-gate"),
    );
    if let Some(p) = position {
        args.insert("position".into(), serde_json::json!(p));
    }
    args
}

#[tokio::test]
async fn move_story_bare_column_move_is_pm_only_1548() {
    for caller in ["dev:claude-code", "designer:claude-code"] {
        let args = move_args(42, "ready", None);
        let err = crate::bridge::move_story::handle_move_story(&args, caller, None)
            .await
            .expect_err(&format!("{caller}: bare column-move must be role-rejected"));
        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "{caller}: bare column-move must 409 workflow-role-not-allowed; got {err}"
        );
    }
}

#[tokio::test]
async fn move_story_positioned_move_is_pm_only_1548() {
    for caller in ["dev:claude-code", "designer:claude-code"] {
        let args = move_args(42, "ready", Some(1));
        let err = crate::bridge::move_story::handle_move_story(&args, caller, None)
            .await
            .expect_err(&format!("{caller}: positioned move must be role-rejected"));
        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "{caller}: positioned move must 409 workflow-role-not-allowed; got {err}"
        );
    }
}

#[tokio::test]
async fn move_story_pm_passes_role_gate_on_both_paths_1548() {
    for position in [None, Some(1)] {
        let args = move_args(42, "ready", position);
        let err = crate::bridge::move_story::handle_move_story(&args, "pm:claude-code", None)
            .await
            .expect_err("pm passes the role-gate then hits refresh-absent");
        assert!(
            !err.contains("workflow-role-not-allowed"),
            "pm must NOT be role-rejected (position={position:?}); got {err}"
        );
        assert!(
            err.contains("requires refresh"),
            "pm passes role-gate → reaches refresh-required (position={position:?}); got {err}"
        );
    }
}

#[tokio::test]
async fn reorder_story_returns_tool_retired_regardless_of_role_1403() {
    let args = reorder_args(42, 1, "ready");
    for caller in [
        "designer:claude-code",
        "dev:claude-code",
        "owner:human",
        "intern:claude-code",
        "pm:claude-code",
    ] {
        let result = handle_reorder_story(&args, caller, None).await;
        let err = result.expect_err(&format!(
            "{caller}: reorder_story is retired; must return tool-retired"
        ));
        assert!(
            err.contains("\"kind\":\"tool-retired\""),
            "{caller}: error must carry canonical kind=tool-retired; got {err}"
        );
        assert!(
            err.contains("\"successor\":\"move_story\""),
            "{caller}: error must cite successor=move_story; got {err}"
        );
        assert!(
            err.contains("\"retired_axis\":\"reorder-tool\""),
            "{caller}: error must cite retired_axis=reorder-tool; got {err}"
        );
        assert!(
            !err.contains("workflow-role-not-allowed"),
            "{caller}: role-gate must NOT fire (retire-gate precedes); got {err}"
        );
    }
}

#[tokio::test]
async fn reorder_story_retire_returns_actionable_successor_path_1403() {
    let args = reorder_args(42, 1, "ready");
    let err = handle_reorder_story(&args, "pm:claude-code", None)
        .await
        .expect_err("retired tool must error");
    assert!(
        err.contains("move_story"),
        "error message must name the successor tool: {err}"
    );
    assert!(
        err.contains("target_state") && err.contains("position"),
        "error message must cite the args caller should use: {err}"
    );
}
