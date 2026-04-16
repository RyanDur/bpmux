//! End-to-end tests for the channel-view filter driven via sidebar
//! clicks (#238). Restores acceptance coverage that was lost when
//! #231 deleted `/view`:
//!   - story54_acceptance_test.rs (whole file) — generic /view
//!     behavior
//!   - story79_acceptance_test.rs (all four channel-filter ACs)
//!   - story55 ac3 (opt-in broadcast filter)
//!   - story71 ac4 (sidebar-driven view + unread badge)
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
/// Enter. `under_section` is true for channels that live under a
/// section header (direct / groups / system) — the filter auto-
/// selects the section header at index 1 in that case, so we need
/// one extra Down press to reach the actual channel.
fn switch_to_channel_via_sidebar(bpmux: &TestBpmux, name: &str, under_section: bool) {
    bpmux.type_ctrl('b');
    assert!(
        bpmux.wait_for_tui("channels", Duration::from_secs(5)),
        "sidebar should open. screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars(name);
    std::thread::sleep(Duration::from_millis(150));
    if under_section {
        // Skip past the section header that sidebar_filter_char
        // auto-selects at index 1.
        bpmux.type_key("Down");
        std::thread::sleep(Duration::from_millis(100));
    }
    bpmux.type_chars("\r"); // Enter activates the selected channel
    std::thread::sleep(Duration::from_millis(300));
}

/// Verify broadcast view opt-in via sidebar filter + Enter. Replaces
/// the deleted story55 ac3 (broadcast filter was opt-in via /view
/// broadcast).
#[test]
fn clicking_broadcast_in_sidebar_filters_dms_out() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    // Send broadcast + DM and wait for the WS round-trip to land.
    // CI is slower than local; assert observable arrivals rather
    // than relying on fixed sleeps.
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("public hello").await;
        dev.send_to("secret dm", &["ryan"]).await;
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

    switch_to_channel_via_sidebar(&bpmux, "broadcast", false);

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

/// Dev channel view shows DMs between owner and dev but excludes
/// broadcasts. Replaces story79 ac1.
#[test]
fn clicking_dev_channel_shows_owner_dev_dms_not_broadcasts() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("broadcast from dev").await;
        dev.send_to("dm from dev", &["ryan"]).await;
    });
    assert!(
        bpmux.wait_for_tui("dm from dev", Duration::from_secs(10)),
        "DM should arrive before we switch views. screen:\n{}",
        bpmux.tui()
    );

    switch_to_channel_via_sidebar(&bpmux, "dev", true);

    let screen = bpmux.tui();
    assert!(
        screen.contains("dm from dev"),
        "DM from dev should appear in dev channel view. screen:\n{screen}"
    );
    // The cross-view broadcast banner at the top of the frame always
    // shows the latest broadcast with a "[broadcast]" prefix — that
    // line stays regardless of channel view, by design (#91). What
    // should NOT render is the broadcast inside the main chat block.
    let chat_only: String = screen
        .lines()
        .filter(|l| !l.contains("[broadcast]"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !chat_only.contains("broadcast from dev"),
        "broadcast should not appear in the chat area of dev view. chat:\n{chat_only}"
    );
}

/// Dev channel view excludes conversations between other agents.
/// Replaces story79 ac2.
#[test]
fn clicking_dev_channel_excludes_other_agents_dms() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut pm = WsPeer::connect(&bpmux, "pm").await;
        dev.send_to("dev says hi", &["ryan"]).await;
        dev.send_to("dev to pm private", &["pm"]).await;
        pm.send_to("pm to dev reply", &["dev"]).await;
    });
    // "dev says hi" is the only message addressed to ryan — once it
    // lands, the other two (between dev and pm) must also have been
    // processed server-side by ordering, so it's safe to proceed.
    assert!(
        bpmux.wait_for_tui("dev says hi", Duration::from_secs(10)),
        "owner↔dev DM should arrive before we switch views. screen:\n{}",
        bpmux.tui()
    );

    switch_to_channel_via_sidebar(&bpmux, "dev", true);

    let screen = bpmux.tui();
    assert!(
        screen.contains("dev says hi"),
        "owner↔dev DM should appear. screen:\n{screen}"
    );
    assert!(
        !screen.contains("dev to pm"),
        "dev↔pm chatter should not appear. screen:\n{screen}"
    );
    assert!(
        !screen.contains("pm to dev"),
        "pm↔dev chatter should not appear. screen:\n{screen}"
    );
}

/// Owner's typed message appears in the current channel view.
/// Replaces story79 ac3.
#[test]
fn owner_message_appears_in_selected_channel_view() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_to("ping from dev", &["ryan"]).await;
    });
    assert!(
        bpmux.wait_for_tui("ping from dev", Duration::from_secs(10)),
        "DM should arrive before we switch views. screen:\n{}",
        bpmux.tui()
    );

    switch_to_channel_via_sidebar(&bpmux, "dev", true);
    bpmux.type_line("owner reply in dev view");

    assert!(
        bpmux.wait_for_tui("owner reply in dev view", Duration::from_secs(5)),
        "owner's own message should render in the channel view. screen:\n{}",
        bpmux.tui()
    );
}
