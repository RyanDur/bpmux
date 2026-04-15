//! Story 58 Acceptance Test: A user can scroll the TUI with the mouse wheel
//!
//! AC1: Mouse wheel up scrolls up through history
//! AC2: Mouse wheel down scrolls back toward latest
//! AC3: Scroll-up indicator appears in status bar
//! AC4: New messages don't yank viewport when scrolled up
//! AC5: Unread indicator shows when scrolled up and new messages arrive

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1+AC3: GIVEN enough messages to overflow viewport
/// WHEN the user scrolls the mouse wheel up
/// THEN earlier messages become visible and status bar shows scroll indicator
#[test]
fn ac1_mouse_scroll_up_shows_history() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        for i in 0..30 {
            dev.send(&format!("message {i}")).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Last message should be visible at bottom
    assert!(
        bpmux.wait_for_tui("message 29", Duration::from_secs(5)),
        "Last message should be visible. Screen:\n{}",
        bpmux.tui()
    );

    // Scroll up with mouse wheel
    bpmux.mouse_scroll_up(10);

    // Should now show scroll indicator
    assert!(
        bpmux.wait_for_tui("scrolled up", Duration::from_secs(5)),
        "Status bar should show scroll indicator. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN user has scrolled up
/// WHEN they scroll the mouse wheel down
/// THEN the view returns toward latest messages
#[test]
fn ac2_mouse_scroll_down_returns_to_latest() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        for i in 0..30 {
            dev.send(&format!("msg {i}")).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Scroll up
    bpmux.mouse_scroll_up(10);
    assert!(
        bpmux.wait_for_tui("scrolled up", Duration::from_secs(5)),
        "Should be scrolled up. Screen:\n{}",
        bpmux.tui()
    );

    // Scroll back down
    bpmux.mouse_scroll_down(20);

    // Should see latest message again
    assert!(
        bpmux.wait_for_tui("msg 29", Duration::from_secs(5)),
        "Latest message should be visible after scrolling down. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4+AC5: GIVEN user is scrolled up
/// WHEN a new message arrives
/// THEN the viewport does NOT yank to bottom AND unread indicator shows
#[test]
fn ac4_new_message_does_not_yank_viewport() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        for i in 0..30 {
            dev.send(&format!("old {i}")).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Scroll up
    bpmux.mouse_scroll_up(10);
    assert!(
        bpmux.wait_for_tui("scrolled up", Duration::from_secs(5)),
        "Should be scrolled up. Screen:\n{}",
        bpmux.tui()
    );

    // New message arrives while scrolled
    rt.block_on(async {
        let mut pm = WsPeer::connect(&bpmux, "pm").await;
        pm.send("new arrival").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    // Should still be scrolled up (not yanked to bottom)
    assert!(
        bpmux.wait_for_tui("scrolled up", Duration::from_secs(5)),
        "Should remain scrolled up after new message. Screen:\n{}",
        bpmux.tui()
    );

    // Should show unread indicator
    assert!(
        bpmux.wait_for_tui("new", Duration::from_secs(5)),
        "Should show unread indicator. Screen:\n{}",
        bpmux.tui()
    );
}
