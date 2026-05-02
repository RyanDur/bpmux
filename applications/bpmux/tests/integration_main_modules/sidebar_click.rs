//! Story #223 acceptance test: clicking a channel name in the
//! sidebar switches the chat view to that channel, without typing
//! /view.
//!
//! The click-to-switch path runs through handle_mouse_click →
//! sidebar_select → channels.current_view. This test proves it
//! works end-to-end from mouse input to rendered view switch.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn clicking_channel_name_in_sidebar_switches_view() {
    let bpmux = TestBpmux::start_named("ryan");

    // Create a second channel by having a peer post in it, so the
    // sidebar has something to click other than the default
    // project/broadcast entries.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "pm").await;
        peer.send("hello from pm").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    // Send the message text so the channel appears. Broadcast view
    // will pick up the peer's message.
    assert!(
        bpmux.wait_for_tui("hello from pm", Duration::from_secs(3)),
        "peer message should arrive. screen:\n{}",
        bpmux.tui()
    );

    // Open the sidebar (Alt+1 is the existing keybind; matches what
    // a user would do today to expose the channels list).
    bpmux.type_key("Alt-1");
    // Give the render loop a beat to lay out the sidebar.
    std::thread::sleep(Duration::from_millis(200));

    // Before the click: status bar shows `[all]` (the default view).
    // This anchors the assertion so we know the click produced the
    // transition, not some other render drift.
    assert!(
        bpmux.tui().contains("[all]"),
        "sanity: status bar should show [all] before click. screen:\n{}",
        bpmux.tui()
    );

    // Click row 3 of the sidebar area. Layout: row 1 = sidebar
    // block border, row 2 = project header, row 3 = first
    // selectable channel (broadcast, which is present because the
    // peer just sent a broadcast message). Column 2 is inside the
    // sidebar content area.
    bpmux.mouse_click(3, 3);
    std::thread::sleep(Duration::from_millis(300));

    // After click: status bar is [broadcast], NOT [all]. This
    // assertion fails if the click wires to the wrong channel or
    // doesn't wire at all.
    assert!(
        bpmux.wait_for_tui("[broadcast]", Duration::from_secs(3)),
        "status bar should switch to [broadcast] after click. screen:\n{}",
        bpmux.tui()
    );
    assert!(
        !bpmux.tui().contains("[all]"),
        "status bar should no longer show [all] after click. screen:\n{}",
        bpmux.tui()
    );
}
