//! End-to-end tests for the channel-view filter driven via sidebar
//! clicks (#238). Restores acceptance coverage that was lost when
//! #231 deleted `/view`:
//!   - story55 ac3 (opt-in broadcast filter)
//!   - story71 ac4 (sidebar-driven view + unread badge)
//!
//! Under #224 v2 the direct-peer rows no longer switch the chat
//! view — they toggle focus. Ryan's direction: scoped 1:1 happens
//! via groups/threads, not DM-peer view filters. The former peer-
//! row view-switch tests (story79 ACs) were intentionally retired
//! with the feature.
//!
//! Filter logic itself is still unit-tested in
//! `src/terminal/ui_tests.rs` against `message_matches_channel`.
//! What these tests protect is the wiring: open sidebar → type to
//! filter → Enter → chat pane re-renders with the right subset.
//!
//! Mechanics: Ctrl+B opens and focuses the sidebar. Typing chars
//! while focused pushes to the sidebar's filter string and auto-
//! selects the first real channel (index 1 when >1 entries per
//! sidebar_filter_char). Enter activates the selection.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// Open the sidebar and keyboard-navigate to `name`, then press
/// Enter. Only usable for non-peer rows (broadcast, self) — peer
/// rows toggle focus rather than switch view under #224 v2.
fn switch_to_channel_via_sidebar(bpmux: &TestBpmux, name: &str) {
    bpmux.type_ctrl('b');
    assert!(
        bpmux.wait_for_tui("channels", Duration::from_secs(5)),
        "sidebar should open. screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars(name);
    std::thread::sleep(Duration::from_millis(150));
    bpmux.type_chars("\r");
    std::thread::sleep(Duration::from_millis(300));
}

/// Verify broadcast view opt-in via sidebar filter + Enter. Replaces
/// the deleted story55 ac3 (broadcast filter was opt-in via /view
/// broadcast).
#[test]
fn clicking_broadcast_in_sidebar_filters_dms_out() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    // Keep the WsPeer alive across the assertions — dropping it at
    // the end of block_on closes the socket, and on slow machines
    // the last in-flight send can be lost in the close handshake.
    let mut dev = rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("public hello").await;
        dev.send_to("secret dm", &["ryan"]).await;
        dev
    });
    assert!(
        bpmux.wait_for_tui("public hello", Duration::from_secs(10)),
        "broadcast should land in default view. screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("secret dm", Duration::from_secs(10)),
        "DM should land in default all-view. screen:\n{}",
        bpmux.tui()
    );
    // Keep `dev` alive during the remaining assertions; drop at end.
    let _ = &mut dev;

    switch_to_channel_via_sidebar(&bpmux, "broadcast");

    // After switching to broadcast: public message stays, DM hides.
    assert!(
        bpmux
            .wait_for_tui_absent("secret dm", Duration::from_secs(5))
            .is_ok(),
        "DM should be filtered out in broadcast view. screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.tui().contains("public hello"),
        "broadcast message should remain. screen:\n{}",
        bpmux.tui()
    );
}

// story79's DM-peer view-switch ACs retired under #224 v2 — peer
// rows now toggle focus, not view. Focus-toggle is unit-tested in
// src/terminal/app_tests.rs; the broadcast-row click above still
// exercises the sidebar → view-switch wiring end-to-end.
