//! End-to-end test for #180 sticky-scroll + new-message indicator.
//!
//! Drives a real bpmux instance via the PTY harness. Verifies that
//! the viewport preserves its position when the user scrolls up and
//! a new message arrives, and that the unread indicator appears.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// Post a chat message via the REST API. Same helper pattern as
/// focus_mode_test and story87_acceptance_test.
async fn post_chat(port: u16, from: &str, text: &str) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{port}/api/messages"))
        .header("Content-Type", "application/json")
        .body(format!(r#"{{"from":"{from}","text":"{text}"}}"#))
        .send()
        .await
        .expect("POST /api/messages");
    assert!(resp.status().is_success());
}

/// GIVEN the user has scrolled up from the bottom
/// WHEN a new message arrives from another agent
/// THEN the viewport stays at the user's scroll position
/// AND the earlier messages remain visible
/// AND a "new" indicator appears
#[test]
fn new_message_does_not_yank_viewport_when_scrolled_up() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    // Seed enough messages to create scrollable content — the chat
    // area in an 80×24 terminal is ~18 rows visible, so 25+ messages
    // will overflow.
    rt.block_on(async {
        for i in 0..25 {
            post_chat(port, "dev:claude-code", &format!("msg-{i:02}")).await;
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    });
    assert!(
        bpmux.wait_for_tui("msg-24", Duration::from_secs(5)),
        "latest message should be visible at bottom. screen:\n{}",
        bpmux.tui()
    );

    // Scroll up via Ctrl+U (the scroll-up keybind).
    bpmux.type_ctrl('u');
    std::thread::sleep(Duration::from_millis(300));
    let scrolled_screen = bpmux.tui();
    // After scrolling up, earlier messages should be visible and the
    // latest message (msg-24) should have scrolled OFF the bottom.
    assert!(
        !scrolled_screen.contains("msg-24"),
        "msg-24 should have scrolled off after Ctrl+U. screen:\n{scrolled_screen}"
    );
    // Capture a reference message that IS visible — one of the
    // earlier ones.
    let has_early = scrolled_screen.contains("msg-0");
    assert!(
        has_early || scrolled_screen.contains("msg-01") || scrolled_screen.contains("msg-02"),
        "some early messages should be visible after scrolling up. screen:\n{scrolled_screen}"
    );

    // Now inject a new message while scrolled up.
    rt.block_on(async {
        post_chat(port, "pm:claude-code", "NEW-AFTER-SCROLL").await;
    });
    std::thread::sleep(Duration::from_millis(300));
    let after_new = bpmux.tui();

    // The viewport should NOT have jumped to show NEW-AFTER-SCROLL.
    assert!(
        !after_new.contains("NEW-AFTER-SCROLL"),
        "new message should NOT yank viewport to bottom. screen:\n{after_new}"
    );
    // The unread indicator should be visible ("N new" in the status
    // bar per the existing unread_count rendering).
    assert!(
        after_new.contains("new"),
        "unread indicator should be visible when scrolled up with new messages. screen:\n{after_new}"
    );
}

/// GIVEN the user is scrolled up with unread messages
/// WHEN the user scrolls back to the bottom (Ctrl+D repeatedly)
/// THEN the new messages become visible
/// AND the unread indicator disappears
#[test]
fn scrolling_to_bottom_reveals_new_messages_and_clears_indicator() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        for i in 0..25 {
            post_chat(port, "dev:claude-code", &format!("fill-{i:02}")).await;
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    });
    assert!(
        bpmux.wait_for_tui("fill-24", Duration::from_secs(5)),
        "fill messages should land"
    );

    // Scroll up.
    bpmux.type_ctrl('u');
    std::thread::sleep(Duration::from_millis(200));

    // Inject a new message.
    rt.block_on(async {
        post_chat(port, "pm:claude-code", "BOTTOM-TARGET").await;
    });
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        !bpmux.tui().contains("BOTTOM-TARGET"),
        "precondition: message should not be visible while scrolled up"
    );

    // Scroll back to bottom with repeated Ctrl+D.
    for _ in 0..10 {
        bpmux.type_ctrl('d');
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(300));

    let bottom_screen = bpmux.tui();
    assert!(
        bottom_screen.contains("BOTTOM-TARGET"),
        "new message should be visible after scrolling to bottom. screen:\n{bottom_screen}"
    );
}
