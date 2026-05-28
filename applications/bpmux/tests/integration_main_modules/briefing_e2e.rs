//! Agent-side E2E test for `save_briefing` + `get_briefing` MCP tool
//! pair (#476 Phase 1 backfill — memory archetype, paired write/read).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — briefing applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn save_briefing_returns_confirmation_with_role_and_bytes() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let content = "## Where I left off\n\nMid-flight on briefing E2E test.\n";
        let result = dev
            .call_tool("save_briefing", serde_json::json!({"content": content}))
            .await
            .expect("save_briefing should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("briefing saved"),
            "happy-path message confirms persistence; got {msg}"
        );
        assert!(msg.contains("dev"), "message names the role; got {msg}");
        assert!(
            msg.contains(&format!("{} bytes", content.len())),
            "message names the size for caller-visibility; got {msg}"
        );

        dev.close().await;
    });
}

#[test]
fn save_briefing_rejects_missing_content() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("save_briefing", serde_json::json!({}))
            .await
            .expect_err("missing content should be rejected");

        assert!(
            err.contains("content"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn save_briefing_get_briefing_roundtrip_preserves_content() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let role = "briefing-roundtrip-test-role";
        let dev = TestBridge::connect(&bpmux, role).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let marker = "BRIEFING_ROUNDTRIP_MARKER_B1";
        let content = format!("## Continuity\n\n{marker}\n\nMid-flight context.");
        dev.call_tool(
            "save_briefing",
            serde_json::json!({"content": content.clone()}),
        )
        .await
        .expect("save_briefing should succeed");

        let briefing = dev
            .call_tool("get_briefing", serde_json::json!({}))
            .await
            .expect("get_briefing should succeed");

        let briefing_text = briefing
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| briefing.to_string());
        assert!(
            briefing_text.contains(marker),
            "content preserved verbatim through real briefing-store; got:\n{briefing_text}"
        );
    });
}

#[test]
fn get_briefing_role_arg_overrides_default() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let pm_marker = "PM_BRIEFING_OVERRIDE_MARKER";
        pm.call_tool(
            "save_briefing",
            serde_json::json!({
                "content": format!("## PM continuity\n\n{pm_marker}")
            }),
        )
        .await
        .expect("save_briefing on pm should succeed");

        let briefing = dev
            .call_tool("get_briefing", serde_json::json!({"role": "pm"}))
            .await
            .expect("get_briefing role=pm should succeed");

        let briefing_text = briefing
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| briefing.to_string());
        assert!(
            briefing_text.contains(pm_marker),
            "role-override surfaces pm's briefing from dev's bridge; got:\n{briefing_text}"
        );

        pm.close().await;
        dev.close().await;
    });
}

/// #1148 — team-doctrine pointer-list surfaces in get_briefing
/// response. Live-bridge e2e test driving the canonical scenario per
/// `feedback_falsifier_load_bearing_test_at_e2e_layer`: agent calls
/// get_briefing, response includes team_doctrine field with the 3
/// canonical doctrine docs, each with mtime when reachable.
#[test]
fn get_briefing_includes_team_doctrine_pointer_list_for_three_canonical_docs() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let response = dev
            .call_tool("get_briefing", serde_json::json!({}))
            .await
            .expect("get_briefing should succeed");

        let response_text = response
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| response.to_string());

        assert!(
            response_text.contains("team_doctrine"),
            "response must surface team_doctrine field per #1148 falsifier; got:\n{response_text}"
        );
        assert!(
            response_text.contains("docs/process-state-machine.md"),
            "team_doctrine must include process-state-machine doc; got:\n{response_text}"
        );
        assert!(
            response_text.contains("docs/handoff-process.md"),
            "team_doctrine must include handoff-process doc; got:\n{response_text}"
        );
        assert!(
            response_text.contains("docs/memory-architecture.md"),
            "team_doctrine must include memory-architecture doc; got:\n{response_text}"
        );

        dev.close().await;
    });
}

#[test]
fn save_briefing_writes_into_session_briefings_dir_not_home() {
    let bpmux = TestBpmux::start();
    let briefings_dir = bpmux.briefings_dir_path();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let marker = "BRIEFINGS_ISOLATION_REGRESSION_MARKER_1563";
        let content = format!("## Isolation regression test\n\n{marker}\n");
        dev.call_tool(
            "save_briefing",
            serde_json::json!({"content": content.clone()}),
        )
        .await
        .expect("save_briefing should succeed");

        let written = briefings_dir.join("dev.md");
        let body = std::fs::read_to_string(&written).unwrap_or_else(|e| {
            panic!(
                "save_briefing must land in session_dir/briefings/dev.md ({}); read error: {e}",
                written.display()
            )
        });
        assert!(
            body.contains(marker),
            "marker round-trips through the session-isolated briefings dir; got:\n{body}"
        );

        dev.close().await;
    });
}

#[test]
fn get_briefing_response_size_under_5kb_per_context_budget_discipline() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let response = dev
            .call_tool("get_briefing", serde_json::json!({}))
            .await
            .expect("get_briefing should succeed");

        let response_text = response
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| response.to_string());

        assert!(
            response_text.len() < 5120,
            "get_briefing total payload must stay context-budget-disciplined (<5KB) per #1148; \
             team_doctrine is pointer-not-content. got {} bytes",
            response_text.len()
        );

        dev.close().await;
    });
}
