//! Story 2 Acceptance Test: Human and AI share a conversation
//!
//! End-to-end test using portable-pty for TUI interaction and rmcp for MCP client.
//! Proves: human types in TUI → message appears, AI sends via MCP → message appears
//! in TUI, AI reads via MCP → gets both messages.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::mcp::McpHttpClient;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn human_and_ai_share_a_conversation() {
    let bpmux = TestBpmux::start_named("ryan");
    let tui = bpmux.tui();
    assert!(tui.contains("bpmux"), "TUI should show bpmux title");
    assert!(tui.contains("solo"), "TUI should show solo status");
    assert!(tui.contains(">"), "TUI should show input prompt");

    bpmux.type_line("hello from human");
    assert!(
        bpmux.wait_for_tui("hello from human", Duration::from_secs(3)),
        "Human message should appear in TUI. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.tui().contains("ryan:"),
        "Sender identity should show. Screen:\n{}",
        bpmux.tui()
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let ai = McpHttpClient::connect(&bpmux).await;

        let tools = ai.tool_names().await;
        assert!(
            tools.iter().any(|t| t == "send_message"),
            "send_message tool missing: {tools:?}"
        );
        assert!(
            tools.iter().any(|t| t == "read_messages"),
            "read_messages tool missing: {tools:?}"
        );

        ai.send("hello from ai").await;

        let content = ai.read_all().await;
        assert!(
            content.contains("hello from human"),
            "read_messages should contain human message. Got: {content}"
        );
        assert!(
            content.contains("hello from ai"),
            "read_messages should contain AI message. Got: {content}"
        );

        ai.close().await;
    });

    assert!(
        bpmux.wait_for_tui("hello from ai", Duration::from_secs(3)),
        "AI message should appear in TUI. Screen:\n{}",
        bpmux.tui()
    );
}
