//! Story 85 Acceptance Test: A user can click sidebar, channels, and help with the mouse.
//!
//! AC1: Clicking the sidebar area toggles it open/close
//! AC2: Clicking a channel in the sidebar switches to that channel
//! AC3: Clicking "? help" opens the help overlay
//! AC4: Keyboard shortcuts still work

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN the sidebar is closed
/// WHEN the user clicks the left edge (column 1)
/// THEN the sidebar opens
#[test]
fn ac1_click_sidebar_toggles_open() {
    let bpmux = TestBpmux::start();

    // Sidebar is closed by default. #183 added a visible ≡ menu
    // glyph at the top-left of the chat area — click it to open.
    // (Previously the click zone was col < 3 on any row; now it's
    // the glyph cell + 3 cols for Fitts's-law forgiveness.)
    // SGR mouse coords are 1-indexed: (1, 1) = top-left cell.
    bpmux.mouse_click(1, 1);

    // Sidebar should now be open — showing "channels" header
    assert!(
        bpmux.wait_for_tui("channels", Duration::from_secs(3)),
        "Sidebar should open on click. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN the sidebar is open with channels
/// WHEN the user clicks a channel name
/// THEN the view switches to that channel
#[test]
fn ac2_click_channel_switches_view() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_to("hello human", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    // Open sidebar with Ctrl+B
    bpmux.type_ctrl('b');
    assert!(
        bpmux.wait_for_tui("channels", Duration::from_secs(3)),
        "Sidebar should open. Screen:\n{}",
        bpmux.tui()
    );

    // Wait for the dev channel to appear in the sidebar
    assert!(
        bpmux.wait_for_tui("dev", Duration::from_secs(3)),
        "Dev channel should appear in sidebar. Screen:\n{}",
        bpmux.tui()
    );

    // The channel list: row 1 = border, row 2 = project root, row 3 = dev channel
    // Click the dev channel line (col 5, row 3)
    bpmux.mouse_click(5, 3);

    // View should switch to dev channel
    assert!(
        bpmux.wait_for_tui("[dev]", Duration::from_secs(3)),
        "View should switch to dev channel on click. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN the status bar shows "? help"
/// WHEN the user clicks the "? help" text
/// THEN the help overlay opens
#[test]
fn ac3_click_help_opens_overlay() {
    let bpmux = TestBpmux::start();

    // "? help" appears at the right edge of the status bar (row 24 in 24-row terminal)
    // Click near the right edge of the status bar
    bpmux.mouse_click(78, 24);

    assert!(
        bpmux.wait_for_tui("Commands:", Duration::from_secs(3)),
        "Help overlay should open on click. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN mouse click features are added
/// WHEN the user uses keyboard shortcuts
/// THEN all existing shortcuts still work
#[test]
fn ac4_keyboard_shortcuts_still_work() {
    let bpmux = TestBpmux::start();

    // Ctrl+B still toggles sidebar
    bpmux.type_ctrl('b');
    assert!(
        bpmux.wait_for_tui("channels", Duration::from_secs(3)),
        "Ctrl+B should still toggle sidebar. Screen:\n{}",
        bpmux.tui()
    );

    // ? still opens help — close sidebar first and verify it's gone
    bpmux.type_ctrl('b');
    bpmux
        .wait_for_tui_absent("channels", Duration::from_secs(5))
        .expect("sidebar should close before opening help");
    bpmux.type_chars("?");
    assert!(
        bpmux.wait_for_tui("Commands:", Duration::from_secs(3)),
        "? should still open help overlay. Screen:\n{}",
        bpmux.tui()
    );
}
