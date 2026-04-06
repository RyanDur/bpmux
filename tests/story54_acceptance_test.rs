//! Story 54 Acceptance Test: A human can switch TUI view with /view [channel]
//!
//! AC1: /view broadcast filters to broadcast-only messages
//! AC2: /view all restores full view
//! AC3: /view <channel> filters to a specific directed channel
//! AC4: /view <nonexistent> shows error, view doesn't change
//! AC5: /view alone lists available channels

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a session with broadcast and directed messages
/// WHEN the human types `/view broadcast`
/// THEN only broadcast messages render and status bar shows [broadcast]
#[test]
fn ac1_view_broadcast_filters_to_broadcast_only() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("public hello").await;
        dev.send_to("secret for human", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    // Both messages should be visible in default [all] view
    assert!(
        bpmux.wait_for_tui("public hello", Duration::from_secs(5)),
        "Broadcast should be visible. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("secret for human", Duration::from_secs(5)),
        "Directed should be visible in [all]. Screen:\n{}",
        bpmux.tui()
    );

    // Switch to broadcast view
    bpmux.type_line("/view broadcast");
    assert!(
        bpmux.wait_for_tui("View set to broadcast", Duration::from_secs(5)),
        "Should confirm view switch. Screen:\n{}",
        bpmux.tui()
    );
    // Directed message should no longer be visible
    assert!(
        !bpmux.wait_for_tui("secret for human", Duration::from_secs(1)),
        "Directed message should be hidden in broadcast view. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN a filtered view
/// WHEN the human types `/view all`
/// THEN all messages re-render and status bar shows [all]
#[test]
fn ac2_view_all_restores_full_view() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("public msg").await;
        dev.send_to("private msg", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    // Filter to broadcast
    bpmux.type_line("/view broadcast");
    assert!(
        bpmux.wait_for_tui("View set to broadcast", Duration::from_secs(5)),
        "Should confirm broadcast view switch. Screen:\n{}",
        bpmux.tui()
    );

    // Switch back to all
    bpmux.type_line("/view all");
    assert!(
        bpmux.wait_for_tui("View set to all", Duration::from_secs(5)),
        "Should confirm all view switch. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("private msg", Duration::from_secs(5)),
        "Directed message should be visible again. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN a directed channel exists (e.g. dev↔human)
/// WHEN the human types `/view dev↔human`
/// THEN only messages in that channel render
#[test]
fn ac3_view_specific_channel() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut pm = WsPeer::connect(&bpmux, "pm").await;
        dev.send("hello everyone").await;
        dev.send_to("hey human from dev", &["human"]).await;
        pm.send_to("hey human from pm", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    // Filter to dev↔human channel
    bpmux.type_line("/view dev\u{2194}human");
    assert!(
        bpmux.wait_for_tui("View set to dev", Duration::from_secs(5)),
        "Should confirm view switch. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("hey human from dev", Duration::from_secs(5)),
        "dev↔human message should be visible. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        !bpmux.wait_for_tui("hey human from pm", Duration::from_secs(1)),
        "pm↔human message should be hidden. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        !bpmux.wait_for_tui("hello everyone", Duration::from_secs(1)),
        "Broadcast message should be hidden. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN a nonexistent channel name
/// WHEN the human types `/view fakechannel`
/// THEN an error message appears and the view doesn't change
#[test]
fn ac4_view_nonexistent_channel_shows_error() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("hello").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    bpmux.type_line("/view fakechannel");
    assert!(
        bpmux.wait_for_tui("no channel matches", Duration::from_secs(5)),
        "Should show error for nonexistent channel. Screen:\n{}",
        bpmux.tui()
    );
    // View should remain [all] (default)
    assert!(
        bpmux.wait_for_tui("[all]", Duration::from_secs(5)),
        "View should not change (still all). Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5: GIVEN available channels exist
/// WHEN the human types `/view` alone
/// THEN the available channel names are listed
#[test]
fn ac5_bare_view_lists_channels() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("broadcast").await;
        dev.send_to("dm", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    bpmux.type_line("/view");
    assert!(
        bpmux.wait_for_tui("Available channels", Duration::from_secs(5)),
        "Should list available channels. Screen:\n{}",
        bpmux.tui()
    );
}
