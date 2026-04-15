//! Story 71 Acceptance Test: A user can navigate channels via a sidebar tree view.
//!
//! AC1: Toggle key opens sidebar showing channels
//! AC2: Selecting a channel sets view + send context
//! AC3: Selecting "all" returns to broadcast
//! AC4: Unread channels show indicator
//! AC5: Esc closes sidebar
//! AC6: Auto-suppressed on narrow terminals

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a user presses Ctrl+B
/// WHEN the sidebar opens
/// THEN a tree view appears showing channels
#[test]
fn ac1_toggle_opens_sidebar() {
    let bpmux = TestBpmux::start();

    // Create a channel by having a peer send a message
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "dev").await;
        peer.send("hello from dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Toggle sidebar with Ctrl+B (\x02)
    bpmux.type_chars("\x02");
    assert!(
        bpmux.wait_for_tui("channels", Duration::from_secs(5)),
        "Sidebar should show 'channels' title. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.tui().contains("all"),
        "Sidebar should show 'all' entry. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN channels have unread messages
/// WHEN the tree renders
/// THEN unread channels show a count badge
#[test]
fn ac4_unread_indicator_in_sidebar() {
    let bpmux = TestBpmux::start();

    // Set view to broadcast so DMs are "unread"
    bpmux.type_line("/view broadcast");
    assert!(
        bpmux.wait_for_tui("broadcast", Duration::from_secs(5)),
        "Should set view. Screen:\n{}",
        bpmux.tui()
    );

    // Send a DM to create an unread channel
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "dev").await;
        peer.send_to("hello human", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Open sidebar — unread channel should show count badge
    bpmux.type_chars("\x02");
    assert!(
        bpmux.wait_for_tui("dev 1", Duration::from_secs(5)),
        "Unread channel should show count badge. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5: GIVEN the sidebar is open
/// WHEN the user presses Esc
/// THEN the sidebar closes
#[test]
fn ac5_esc_closes_sidebar() {
    let bpmux = TestBpmux::start();

    // Open sidebar
    bpmux.type_chars("\x02");
    assert!(
        bpmux.wait_for_tui("channels", Duration::from_secs(5)),
        "Sidebar should open. Screen:\n{}",
        bpmux.tui()
    );

    // Close with Esc. The original test slept 500ms here with the
    // comment "at minimum, verify no crash" — it never actually
    // asserted the sidebar closed. That latent coverage gap is out
    // of scope for #198 (would be addressed by region-scoped
    // locators per the designer's test API proposal). The type_chars
    // call itself is the crash-exercise path.
    bpmux.type_chars("\x1b");
}
