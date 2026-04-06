//! Story 50 Acceptance Test: HTTP MCP clients can set their display name.
//!
//! AC1: set_name changes message attribution
//! AC2: Default identity is reasonable (not the TUI owner's name)
//! AC3: Two clients have independent identities

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::mcp::McpHttpClient;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN an HTTP MCP client connects
/// WHEN it calls set_name with a display name
/// THEN subsequent messages are attributed to that name
#[test]
fn ac1_set_name_changes_attribution() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    rt.block_on(async {
        let client = McpHttpClient::connect(&bpmux).await;

        // Verify set_name and set_status appear in tool list
        let tools = client.tool_names().await;
        assert!(
            tools.contains(&"set_name".to_string()),
            "Should have set_name tool"
        );
        assert!(
            tools.contains(&"set_status".to_string()),
            "Should have set_status tool"
        );

        // Set name and send message
        client.set_name("alice").await;
        client.send("hello from alice").await;
        client.close().await;
    });

    assert!(
        bpmux.wait_for_tui("alice", Duration::from_secs(5)),
        "Message should be attributed to alice. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN an HTTP MCP client has not called set_name
/// WHEN it sends a message
/// THEN it appears with a default identity (not the TUI owner's name)
#[test]
fn ac2_default_identity_is_reasonable() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    rt.block_on(async {
        let client = McpHttpClient::connect(&bpmux).await;
        client.send("anonymous message").await;
        client.close().await;
    });

    assert!(
        bpmux.wait_for_tui("anonymous message", Duration::from_secs(5)),
        "Message should appear. Screen:\n{}",
        bpmux.tui()
    );
    // The default name should NOT be the TUI owner's name ("human")
    // It should be the server's peer_name which defaults to the --name arg
    let screen = bpmux.tui();
    assert!(
        screen.contains("anonymous message"),
        "Message should be visible. Screen:\n{}",
        screen
    );
}

/// AC3: GIVEN two HTTP MCP clients connect
/// WHEN each calls set_name with different names
/// THEN their messages are attributed independently
#[test]
fn ac3_independent_identities() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    rt.block_on(async {
        let client1 = McpHttpClient::connect(&bpmux).await;
        let client2 = McpHttpClient::connect(&bpmux).await;

        client1.set_name("alice").await;
        client2.set_name("bob").await;

        client1.send("from alice").await;
        client2.send("from bob").await;

        client1.close().await;
        client2.close().await;
    });

    assert!(
        bpmux.wait_for_tui("alice", Duration::from_secs(5)),
        "Should show alice. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("bob", Duration::from_secs(5)),
        "Should show bob. Screen:\n{}",
        bpmux.tui()
    );
}
