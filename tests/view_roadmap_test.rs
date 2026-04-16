//! view_roadmap acceptance test: an agent calls the tool; the
//! roadmap surfaces in Ryan's TUI AND the tool returns the roadmap
//! text to the calling agent (#246 — the original #226 tool
//! returned only a success string, breaking the "agent surfaces
//! roadmap content in chat" use case).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_roadmap_returns_content_and_opens_overlay() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        let result = pm
            .call_tool("view_roadmap", serde_json::json!({}))
            .await
            .expect("view_roadmap should succeed");
        let tool_response = result.as_str().unwrap_or("");
        // The response now carries the ROADMAP.md body so the agent
        // can share it in chat. The harness sets BPMUX_ROOT to the
        // repo root, so ROADMAP.md is readable.
        assert!(
            tool_response.contains("bpmux Roadmap") || tool_response.contains("Roadmap"),
            "tool should return roadmap text, got: {tool_response}"
        );
        assert!(
            tool_response.len() > 50,
            "tool response should contain substantial roadmap text, not just a status line. got {} chars",
            tool_response.len()
        );

        // TUI side effect unchanged — overlay opens so Ryan can read it.
        let appeared = bpmux.wait_for_tui("Roadmap", Duration::from_secs(5))
            || bpmux.wait_for_tui("roadmap", Duration::from_secs(5));
        assert!(
            appeared,
            "roadmap text should appear in TUI. screen:\n{}",
            bpmux.tui()
        );

        pm.close().await;
    });
}

#[test]
fn view_roadmap_returns_meaningful_error_when_file_missing() {
    // Point BPMUX_ROOT at a tempdir with no ROADMAP.md. The tool
    // must not return empty silently — callers need to distinguish
    // "here's the roadmap" from "roadmap unavailable" so they can
    // tell Ryan something useful. Both the TUI process (via
    // start_with_env) and the bridge subprocess (via
    // connect_with_env) need the override, because fetch_roadmap
    // runs in whichever process serves the MCP call.
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("utf-8 tempdir");
    let bpmux = TestBpmux::start_with_env("ryan", "BPMUX_ROOT", root);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_env(&bpmux, "pm", &[("BPMUX_ROOT", root)]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        let result = pm
            .call_tool("view_roadmap", serde_json::json!({}))
            .await
            .expect("view_roadmap should succeed even when file missing");
        let response = result.as_str().unwrap_or("");
        assert!(
            response.contains("unavailable") || response.contains("cannot read"),
            "missing roadmap must produce a clear message, got: {response}"
        );

        pm.close().await;
    });
}
