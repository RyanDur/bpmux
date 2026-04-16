//! Acceptance test for the `share_message` MCP tool (#174). An
//! agent calls the tool; a shared message appears in the target
//! scope with the provenance marker the TUI's `/share` also uses.
//!
//! The privacy gate is covered at unit level in
//! `src/bridge/tests.rs` (pure helpers) and through the existing
//! /share app_tests + #247's explicit_privacy agent parity; this
//! file exercises the end-to-end MCP wiring.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::bridge::TestBridge;
use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

#[tokio::test]
async fn share_message_form2_posts_fresh_text_with_provenance() {
    // Form 2: agent provides `text` AND `scope`. Result should
    // appear in the session with `[Shared by <agent>] <text>` prefix.
    let bpmux = TestBpmux::start_named("ryan");
    let pm = TestBridge::connect(&bpmux, "pm").await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let result = pm
        .call_tool(
            "share_message",
            serde_json::json!({"scope": "all", "text": "coordinating"}),
        )
        .await
        .expect("share_message form 2 should succeed");
    let response = result.as_str().unwrap_or("");
    assert!(
        response.contains("shared fresh message"),
        "tool response should confirm form-2 share, got: {response}"
    );

    // The shared text surfaces in the bpmux session and thus the
    // TUI chat area. `[Shared by pm]` is the provenance marker.
    assert!(
        bpmux.wait_for_tui("[Shared by pm]", Duration::from_secs(5)),
        "provenance marker should appear. screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("coordinating", Duration::from_secs(5)),
        "shared text should appear. screen:\n{}",
        bpmux.tui()
    );

    pm.close().await;
}

#[tokio::test]
async fn share_message_form1_refuses_to_widen_explicit_privacy_dm() {
    // #247 makes agent DMs set explicit_privacy=true. The share
    // tool must refuse to re-share them, matching the TUI /share
    // privacy guarantee.
    let bpmux = TestBpmux::start_named("ryan");
    let pm = TestBridge::connect(&bpmux, "pm").await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // An external peer sends a DM to pm — this is the most recent
    // visible chat in pm's view, and it carries explicit_privacy.
    let mut dev = WsPeer::connect(&bpmux, "dev").await;
    dev.send_to("secret dm for pm", &["pm"]).await;
    tokio::time::sleep(Duration::from_millis(400)).await;

    // pm tries to /share all — should refuse.
    let result = pm
        .call_tool("share_message", serde_json::json!({"scope": "all"}))
        .await;
    // Tool returns Err on refusal per rmcp convention (the handler
    // uses Err for guard rejections).
    assert!(
        result.is_err(),
        "share_message must refuse to widen an explicit-privacy DM, got Ok({result:?})"
    );
    let err = format!("{:?}", result.unwrap_err()).to_lowercase();
    assert!(
        err.contains("private") || err.contains("paraphrase"),
        "refusal message should mention privacy; got: {err}"
    );

    pm.close().await;
}

#[tokio::test]
async fn share_message_invalid_scope_returns_error() {
    let bpmux = TestBpmux::start_named("ryan");
    let pm = TestBridge::connect(&bpmux, "pm").await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let result = pm
        .call_tool(
            "share_message",
            serde_json::json!({"scope": "", "text": "hi"}),
        )
        .await;
    assert!(
        result.is_err(),
        "empty scope must error, got Ok({result:?})"
    );

    pm.close().await;
}
