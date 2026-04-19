//! End-to-end tests for `simulate_key` firing registered keybindings
//! (#310). The existing simulate_key_modifiers_test covers Alt+1 →
//! sidebar; this file extends coverage to the keys designer reported
//! as "routes as text instead of triggering the handler": `?`, `/`,
//! Ctrl+B, Ctrl+K, Esc. If any of these actually routes to the input
//! buffer instead of firing its handler, the assertion fails and
//! surfaces the exact keybinding where parity with Ryan's keystrokes
//! breaks.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

fn post_key(bpmux: &TestBpmux, body: serde_json::Value) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let resp = bpmux.authed_post("/api/simulate/key", body).await;
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "simulate_key endpoint should accept"
        );
    });
}

fn wait_ready(bpmux: &TestBpmux) {
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
}

#[test]
fn question_mark_opens_help_overlay_when_input_is_empty() {
    let bpmux = TestBpmux::start();
    wait_ready(&bpmux);

    post_key(&bpmux, serde_json::json!({"key": "?"}));

    // Help overlay renders a distinctive block with "Keybindings" or
    // similar. The renderer places it over chat, so its signature is
    // whatever text the help-overlay component draws. Checking for
    // "help" alone is too loose (the hint bar also contains it);
    // wait for content that only appears in the overlay.
    bpmux
        .wait_until("help overlay to appear", Duration::from_secs(5), || {
            let tui = bpmux.tui();
            tui.contains("help") && tui.contains("Commands:")
        })
        .expect("? should open help overlay when input is empty");
}

#[test]
fn ctrl_b_toggles_sidebar() {
    let bpmux = TestBpmux::start();
    wait_ready(&bpmux);

    let before = bpmux.tui().contains("channels");
    post_key(
        &bpmux,
        serde_json::json!({"key": "b", "modifiers": ["ctrl"]}),
    );

    bpmux
        .wait_until("sidebar state to flip", Duration::from_secs(5), || {
            bpmux.tui().contains("channels") != before
        })
        .expect("Ctrl+B should toggle sidebar");
}

#[test]
fn ctrl_1_toggles_sidebar() {
    let bpmux = TestBpmux::start();
    wait_ready(&bpmux);

    let before = bpmux.tui().contains("channels");
    post_key(
        &bpmux,
        serde_json::json!({"key": "1", "modifiers": ["ctrl"]}),
    );

    bpmux
        .wait_until("sidebar state to flip", Duration::from_secs(5), || {
            bpmux.tui().contains("channels") != before
        })
        .expect("Ctrl+1 should toggle sidebar");
}

#[test]
fn ctrl_k_opens_command_palette() {
    let bpmux = TestBpmux::start();
    wait_ready(&bpmux);

    post_key(
        &bpmux,
        serde_json::json!({"key": "k", "modifiers": ["ctrl"]}),
    );

    bpmux
        .wait_until("palette to open", Duration::from_secs(5), || {
            bpmux.tui().contains("actions")
        })
        .expect("Ctrl+K should open the command palette");
}

#[test]
fn esc_closes_opened_help_overlay() {
    let bpmux = TestBpmux::start();
    wait_ready(&bpmux);

    post_key(&bpmux, serde_json::json!({"key": "?"}));
    bpmux
        .wait_until("help overlay", Duration::from_secs(5), || {
            let tui = bpmux.tui();
            tui.contains("help") && tui.contains("Commands:")
        })
        .expect("help should open first");

    post_key(&bpmux, serde_json::json!({"key": "esc"}));
    bpmux
        .wait_until("help overlay to close", Duration::from_secs(5), || {
            !bpmux.tui().contains("Commands:")
        })
        .expect("Esc should close help overlay");
}
