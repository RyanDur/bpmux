#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::{
    connected_ws_sink_for_test, connected_ws_sink_with_capture, test_ci_state_store_green,
};
use crate::board::domain::StoryType;
use crate::session::velocity::classify_story;
use crate::transport::protocol::ClientMessage;

#[test]
fn role_from_name_extracts_short_role() {
    assert_eq!(role_from_name("dev:claude-code"), "dev");
    assert_eq!(role_from_name("pm"), "pm");
    assert_eq!(role_from_name("designer:cursor"), "designer");
    assert_eq!(role_from_name("ryan:human"), "ryan");
}

#[test]
fn require_role_allows_matching_role() {
    assert!(require_role("dev:claude-code", &["dev"], "start").is_ok());
    assert!(require_role("pm", &["pm"], "label").is_ok());
    assert!(require_role("ryan:human", &["ryan"], "accept").is_ok());
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
    let result = require_role("dev:claude-code", &["ryan"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to ryan"));

    let result = require_role("pm:claude-code", &["ryan"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to ryan"));
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
fn require_role_rejects_ryan_from_close_iteration() {
    let result = require_role("ryan:human", &["pm"], "close iteration");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
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
fn build_board_invalidate_signal_is_system_typed_broadcast() {
    let signal = build_board_invalidate_signal();
    let ClientMessage::Send {
        to,
        addressee,
        text,
        message_type,
    } = signal
    else {
        panic!("expected Send variant");
    };
    assert!(
        to.is_none(),
        "to must be None so every connected TUI refreshes, not just one"
    );
    assert!(
        addressee.is_none(),
        "addressee must be None — not an attention signal, just a state-change nudge"
    );
    assert_eq!(
        text, BOARD_INVALIDATE_MARKER,
        "text must be the exact marker constant so event.rs can structurally match it"
    );
    assert_eq!(
        message_type.as_deref(),
        Some("system"),
        "message_type must be 'system' so the TUI suppresses chat rendering"
    );
}

#[test]
fn close_story_is_pm_only_gate() {
    for caller in ["dev:claude-code", "designer:claude-code", "ryan:human"] {
        let result = require_role(caller, &["pm"], "close");
        assert!(
            result.is_err(),
            "{caller} should not pass close_story permission gate"
        );
        assert!(result.unwrap_err().contains("restricted to pm"));
    }
}

#[test]
fn close_story_allows_pm() {
    assert!(require_role("pm:claude-code", &["pm"], "close").is_ok());
}

#[tokio::test]
async fn comment_on_story_rejects_empty_body() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(42));
    args.insert("body".to_string(), serde_json::json!(""));
    let result = handle_comment_on_story(&args, "dev:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "empty body must be rejected: got {result:?}"
    );
    assert!(result.unwrap_err().contains("must not be empty"));
}

#[tokio::test]
async fn comment_on_story_rejects_whitespace_only_body() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(42));
    args.insert("body".to_string(), serde_json::json!("   \n  "));
    let result = handle_comment_on_story(&args, "dev:claude-code", &ws_tx, None).await;
    assert!(result.is_err(), "whitespace-only body must be rejected");
}

#[tokio::test]
async fn comment_on_story_rejects_when_refresh_absent() {
    let ws_tx = tokio::sync::Mutex::new(None);
    for caller in ["dev:claude-code", "pm:claude-code", "designer:claude-code"] {
        let mut args = serde_json::Map::new();
        args.insert("number".to_string(), serde_json::json!(42));
        args.insert("body".to_string(), serde_json::json!("a real comment"));
        let result = handle_comment_on_story(&args, caller, &ws_tx, None).await;
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
fn require_role_rejects_non_designer_from_desk_check_operations() {
    for caller in ["dev:claude-code", "pm:claude-code", "ryan:human"] {
        let result = require_role(caller, &["designer"], "pass desk-check");
        assert!(
            result.is_err(),
            "{caller} should not pass desk-check; got: {result:?}"
        );
        assert!(result.unwrap_err().contains("restricted to designer"));
    }
}
