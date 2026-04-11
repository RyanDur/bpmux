//! End-to-end test for #170's `modifiers` support on
//! `/api/simulate/key`.
//!
//! The unit tests in `src/transport/rest_tests.rs` verify that the
//! handler EMITS a `crossterm::event::KeyEvent` with the correct
//! modifier bits. This test closes the other half of the AC from
//! #170: "AND the TUI's next frame reflects whatever Alt+1 is bound
//! to (e.g. sidebar toggle)". It drives the REAL bpmux process
//! through a PTY, POSTs to `/api/simulate/key` with a modifier, and
//! asserts on the captured screen — so the endpoint → event pipeline
//! → event dispatcher → render loop path is exercised end-to-end.
//!
//! Named in behavior terms because that's what the test actually
//! verifies, per `feedback_test_name_matches_content.md`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// Whether the sidebar is currently rendered. The sidebar draws a
/// bordered block with " channels " as its title (see
/// `src/terminal/sidebar.rs`). When the sidebar is closed it
/// contributes 0 columns to the layout and the title is gone —
/// so the literal string "channels" in the title position is a
/// clean, narrow signature that only reacts to real sidebar
/// state changes. Contrast with checking the chat pane's own
/// " bpmux " title, which is always visible regardless of
/// sidebar state.
fn sidebar_open(tui: &str) -> bool {
    tui.contains("channels")
}

/// GIVEN bpmux running with its default sidebar-open state
/// WHEN an agent POSTs `{"key":"1","modifiers":["alt"]}` to
///   `/api/simulate/key` (the Alt+1 keybinding is the sidebar
///   toggle, per `src/terminal/event_dispatch.rs`)
/// THEN the TUI's next frame reflects a sidebar state change
/// AND a follow-up POST with the same payload restores the
///   original state (toggle-back sanity check)
#[test]
fn alt_1_via_simulate_key_endpoint_toggles_the_sidebar() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

    // Let the initial TUI render settle.
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    let before = sidebar_open(&bpmux.tui());

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://127.0.0.1:{port}/api/simulate/key"))
            .json(&serde_json::json!({"key": "1", "modifiers": ["alt"]}))
            .send()
            .await
            .expect("POST /api/simulate/key");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "endpoint should accept alt modifier"
        );
    });

    // Settle before comparing — ratatui redraws on next event loop tick.
    std::thread::sleep(Duration::from_millis(300));
    let after = sidebar_open(&bpmux.tui());

    assert_ne!(
        before,
        after,
        "sidebar open-state should have flipped after Alt+1 via endpoint. \
         before={before}, after={after}. screen:\n{}",
        bpmux.tui()
    );

    // Toggle back — sanity check that the state is symmetric and the
    // first change wasn't a fluke.
    rt.block_on(async {
        let client = reqwest::Client::new();
        client
            .post(format!("http://127.0.0.1:{port}/api/simulate/key"))
            .json(&serde_json::json!({"key": "1", "modifiers": ["alt"]}))
            .send()
            .await
            .expect("POST /api/simulate/key");
    });
    std::thread::sleep(Duration::from_millis(300));
    let restored = sidebar_open(&bpmux.tui());
    assert_eq!(
        before, restored,
        "second Alt+1 should restore original sidebar state. \
         expected={before}, got={restored}"
    );
}

/// Regression guard for the original #170 silent-ignore bug: if the
/// `modifiers` field is dropped, the endpoint sends plain `1` instead
/// of Alt+1, which is not bound to any action — the sidebar would
/// NOT toggle, and the test above would pass only when the modifier
/// is correctly applied. This test ensures plain `1` (no modifier)
/// does NOT toggle the sidebar, proving the modifier is what's
/// driving the change.
#[test]
fn plain_1_via_simulate_key_endpoint_does_not_toggle_sidebar() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    let before = sidebar_open(&bpmux.tui());

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://127.0.0.1:{port}/api/simulate/key"))
            .json(&serde_json::json!({"key": "1"}))
            .send()
            .await
            .expect("POST /api/simulate/key");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
    });
    std::thread::sleep(Duration::from_millis(300));
    let after = sidebar_open(&bpmux.tui());

    // Plain '1' with no modifier types into the input field, which
    // does NOT change sidebar open-state. If this test ever starts
    // failing, it means either (a) the modifier got silently applied
    // (the #170 bug returning), or (b) plain '1' gained a new
    // keybinding that toggles the sidebar.
    assert_eq!(
        before, after,
        "plain '1' should NOT toggle sidebar (would indicate silent modifier injection). \
         before={before}, after={after}"
    );
}
