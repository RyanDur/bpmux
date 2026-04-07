//! Story 63 Acceptance Test: A user can see how many unread messages arrived
//! while scrolled up.
//!
//! AC1: Scrolled up + new messages → unread indicator
//! AC2: At bottom → no indicator
//! AC3: Scroll to bottom → indicator disappears
//! AC4: Count increments with each new message

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a user is scrolled up
/// WHEN new messages arrive
/// THEN an unread indicator appears
#[test]
fn ac1_unread_indicator_when_scrolled_up() {
    let bpmux = TestBpmux::start();

    // Fill chat with messages so we can scroll (needs enough to overflow viewport)
    for i in 0..25 {
        bpmux.type_line(&format!("fill message {i}"));
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(200));

    // Scroll up
    bpmux.mouse_scroll_up(5);
    std::thread::sleep(Duration::from_millis(200));

    // Send a message from a peer so new content arrives
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "bot").await;
        peer.send("unread test message").await;
        // Give time for the message to arrive and render
        tokio::time::sleep(Duration::from_secs(1)).await;
    });

    assert!(
        bpmux.wait_for_tui("new message", Duration::from_secs(5)),
        "Should show unread indicator. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN a user is at the bottom
/// WHEN new messages arrive
/// THEN no unread indicator appears
#[test]
fn ac2_no_indicator_at_bottom() {
    let bpmux = TestBpmux::start();

    // Send a message from a peer while we're at the bottom
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "bot").await;
        peer.send("live message").await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    });

    assert!(
        bpmux.wait_for_tui("live message", Duration::from_secs(5)),
        "Message should appear. Screen:\n{}",
        bpmux.tui()
    );
    // Should NOT show unread indicator
    let screen = bpmux.tui();
    assert!(
        !screen.contains("↓"),
        "Should not show unread indicator at bottom. Screen:\n{}",
        screen
    );
}

/// AC3: GIVEN unread indicator is visible
/// WHEN the user scrolls to the bottom
/// THEN the indicator disappears
#[test]
fn ac3_scroll_to_bottom_clears_indicator() {
    let bpmux = TestBpmux::start();

    // Fill and scroll up (needs enough to overflow viewport)
    for i in 0..25 {
        bpmux.type_line(&format!("scroll msg {i}"));
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(200));
    bpmux.mouse_scroll_up(5);
    std::thread::sleep(Duration::from_millis(200));

    // Send from peer
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "bot").await;
        peer.send("trigger unread").await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    });

    assert!(
        bpmux.wait_for_tui("new message", Duration::from_secs(5)),
        "Unread indicator should appear. Screen:\n{}",
        bpmux.tui()
    );

    // Scroll back to bottom
    bpmux.mouse_scroll_down(20);
    std::thread::sleep(Duration::from_millis(500));

    let screen = bpmux.tui();
    assert!(
        !screen.contains("↓"),
        "Unread indicator should disappear after scrolling to bottom. Screen:\n{}",
        screen
    );
}
