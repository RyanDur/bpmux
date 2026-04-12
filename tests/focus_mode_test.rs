//! End-to-end test for #181 focus mode part A — display filter by
//! role.
//!
//! Unit tests for `parse_focus_args` and `in_focus` live in
//! `src/terminal/commands_focus.rs::tests`. This file is the behavior
//! layer: real PTY-hosted bpmux, messages posted via the REST message
//! API, `/focus` typed as a user, captured screen inspected.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// Post a chat message to the bpmux session via the REST API.
/// Matches the pattern used by `story87_acceptance_test` and the
/// production plugin hook.
async fn post_chat(port: u16, from: &str, text: &str) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{port}/api/messages"))
        .header("Content-Type", "application/json")
        .body(format!(r#"{{"from":"{from}","text":"{text}"}}"#))
        .send()
        .await
        .expect("POST /api/messages");
    assert!(
        resp.status().is_success(),
        "post_chat failed: {}",
        resp.status()
    );
}

/// GIVEN the session has messages from dev, pm, and designer
/// WHEN the user runs `/focus dev,pm`
/// THEN messages from designer disappear from the viewport
/// AND messages from dev and pm are still visible
#[test]
fn focus_hides_messages_from_roles_outside_the_focused_set() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        post_chat(port, "dev:claude-code", "dev-says-alpha").await;
        post_chat(port, "pm:claude-code", "pm-says-beta").await;
        post_chat(port, "designer:claude-code", "designer-says-gamma").await;
    });
    assert!(
        bpmux.wait_for_tui("designer-says-gamma", Duration::from_secs(5)),
        "all three messages should be present before filter. screen:\n{}",
        bpmux.tui()
    );

    bpmux.type_line("/focus dev,pm");
    // Settle a moment for the filter to apply and the viewport to re-render.
    std::thread::sleep(Duration::from_millis(400));

    let screen = bpmux.tui();
    assert!(
        screen.contains("dev-says-alpha"),
        "dev message should still be visible. screen:\n{screen}"
    );
    assert!(
        screen.contains("pm-says-beta"),
        "pm message should still be visible. screen:\n{screen}"
    );
    assert!(
        !screen.contains("designer-says-gamma"),
        "designer message should be hidden by the focus filter. screen:\n{screen}"
    );
    // Status bar indicator.
    assert!(
        screen.contains("[focus: dev,pm]"),
        "status bar should show focus indicator. screen:\n{screen}"
    );
}

/// GIVEN the focus filter has hidden messages
/// WHEN the user runs `/focus off`
/// THEN hidden messages reappear
/// AND the status bar no longer shows the focus indicator
#[test]
fn focus_off_restores_all_hidden_messages() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        post_chat(port, "dev:claude-code", "dev-alpha").await;
        post_chat(port, "designer:claude-code", "designer-gamma").await;
    });
    assert!(
        bpmux.wait_for_tui("designer-gamma", Duration::from_secs(5)),
        "both messages should be visible. screen:\n{}",
        bpmux.tui()
    );

    bpmux.type_line("/focus dev");
    std::thread::sleep(Duration::from_millis(300));
    let filtered = bpmux.tui();
    assert!(
        !filtered.contains("designer-gamma"),
        "designer message should be hidden. screen:\n{filtered}"
    );

    bpmux.type_line("/focus off");
    std::thread::sleep(Duration::from_millis(300));
    let restored = bpmux.tui();
    assert!(
        restored.contains("designer-gamma"),
        "designer message should reappear after /focus off. screen:\n{restored}"
    );
    assert!(
        !restored.contains("[focus:"),
        "status bar should no longer show focus indicator. screen:\n{restored}"
    );
}
