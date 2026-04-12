//! Story 60 Acceptance Test: A first-time user understands the basics within
//! 60 seconds of starting bpmux.
//!
//! AC3: Help keybinding shows overlay
//! AC4: Any key dismisses help overlay

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// AC3: GIVEN a user is in the TUI
/// WHEN they press `?` with empty input
/// THEN a help overlay appears
#[test]
fn ac3_help_keybinding_shows_overlay() {
    let bpmux = TestBpmux::start();
    // Press ? to show help overlay
    bpmux.type_chars("?");
    assert!(
        bpmux.wait_for_tui("press any key to close", Duration::from_secs(5)),
        "Should show help overlay. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN the help overlay is visible
/// WHEN the user presses any key
/// THEN the overlay closes
#[test]
fn ac4_any_key_dismisses_help() {
    let bpmux = TestBpmux::start();
    bpmux.type_chars("?");
    assert!(
        bpmux.wait_for_tui("press any key to close", Duration::from_secs(5)),
        "Help should be visible. Screen:\n{}",
        bpmux.tui()
    );
    // Press a key to dismiss
    bpmux.type_chars("x");
    // Wait briefly, then verify overlay is gone
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !bpmux.tui().contains("press any key to close"),
        "Help overlay should be dismissed. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5 (added 2026-04-11 for #164 re-delivery): the `?` overlay must
/// include short descriptions alongside command names, not just names.
/// GIVEN a user is in the TUI
/// WHEN they press `?` with empty input
/// THEN the rendered overlay contains at least one command description
/// The #164 original delivery regressed the overlay to names-only,
/// which Ryan correctly rejected. This test guards against that
/// regression at the real-render level — it's the end-to-end
/// companion to `commands_help::tests::format_command_list_2col_*`
/// unit tests. If someone lowers the `format_command_list(2)` output
/// to names only, OR if `render_help_overlay` stops calling
/// `format_command_list(2)`, OR if the rendered frame drops the
/// description text for any other reason, this test catches it
/// because the assertion is on the actually-drawn screen buffer.
#[test]
fn ac5_help_overlay_shows_command_descriptions() {
    let bpmux = TestBpmux::start();
    bpmux.type_chars("?");
    assert!(
        bpmux.wait_for_tui("press any key to close", Duration::from_secs(5)),
        "Help overlay should be visible. Screen:\n{}",
        bpmux.tui()
    );
    // The overlay should contain at least one command description token
    // that does NOT appear in any command name. Tokens chosen carefully
    // to avoid collision with slash-command names: "propagate" only in
    // /share's desc; "toggle" in /all and /debug; "filter" in /all.
    // None appear as command names themselves, so a "names-only"
    // regression would contain zero of them.
    let screen = bpmux.tui();
    let has_description =
        screen.contains("propagate") || screen.contains("toggle") || screen.contains("filter");
    assert!(
        has_description,
        "overlay should render at least one command description token \
         (propagate / toggle / filter). a names-only regression has \
         none of these. screen:\n{screen}"
    );
}
