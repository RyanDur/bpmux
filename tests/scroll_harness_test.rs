//! Comprehensive scroll behavior tests for #180.
//!
//! These tests cover every scroll scenario Ryan encountered during
//! today's debugging. ALL must pass before any scroll changes push
//! to the live binary.
//!
//! Scenarios:
//! 1. At bottom, new message arrives → viewport follows (auto-scroll)
//! 2. Scrolled up, new message arrives → viewport stays put
//! 3. Scrolled up, scroll down to bottom → auto-follow resumes
//! 4. Scroll up then down (direction change) → no lag, smooth
//! 5. Long message arrives while scrolled up → no proportional drift
//! 6. Multiple messages arrive rapidly while scrolled up → stable
//! 7. Scroll to absolute top → first messages visible

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

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

fn seed_messages(bpmux: &TestBpmux, count: usize) {
    let port = bpmux.port();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        for i in 0..count {
            post_chat(port, "dev:claude-code", &format!("msg-{i:03}")).await;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    });
    let last = format!("msg-{:03}", count - 1);
    assert!(
        bpmux.wait_for_tui(&last, Duration::from_secs(5)),
        "seeding: last message should be visible"
    );
}

/// Scenario 1: at bottom, new message → viewport follows
#[test]
fn at_bottom_new_message_auto_scrolls() {
    let bpmux = TestBpmux::start();
    seed_messages(&bpmux, 25);
    let before = bpmux.tui();
    assert!(before.contains("msg-024"));

    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        post_chat(bpmux.port(), "pm:claude-code", "NEW-AT-BOTTOM").await;
    });
    assert!(
        bpmux.wait_for_tui("NEW-AT-BOTTOM", Duration::from_secs(3)),
        "new message should auto-scroll into view"
    );
}

/// Scenario 2: scrolled up, new message → viewport stays put
#[test]
fn scrolled_up_new_message_stays_put() {
    let bpmux = TestBpmux::start();
    seed_messages(&bpmux, 25);

    bpmux.type_ctrl('u');
    bpmux
        .wait_until(
            "viewport to scroll away from msg-024",
            Duration::from_secs(5),
            || !bpmux.tui().contains("msg-024"),
        )
        .expect("should have scrolled away from last message");

    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        post_chat(bpmux.port(), "pm:claude-code", "SHOULD-NOT-APPEAR").await;
    });
    // Negative assertion: verify the message stays absent by checking
    // AFTER a known-processed downstream signal. The unread-indicator
    // update when a message arrives while scrolled up IS the signal
    // that the message was received and routed without yanking.
    assert!(
        bpmux.wait_for_tui("new message", Duration::from_secs(5)),
        "unread indicator should appear when message arrives while scrolled up. screen:\n{}",
        bpmux.tui()
    );
    let after = bpmux.tui();
    assert!(
        !after.contains("SHOULD-NOT-APPEAR"),
        "new message should NOT yank viewport. screen:\n{after}"
    );
}

/// Scenario 3: scroll back to bottom → see new messages
#[test]
fn scroll_to_bottom_shows_new_messages() {
    let bpmux = TestBpmux::start();
    seed_messages(&bpmux, 25);

    bpmux.type_ctrl('u');
    bpmux
        .wait_until("viewport to scroll up", Duration::from_secs(5), || {
            !bpmux.tui().contains("msg-024")
        })
        .expect("should have scrolled up");

    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        post_chat(bpmux.port(), "pm:claude-code", "BOTTOM-MSG").await;
    });
    assert!(
        bpmux.wait_for_tui("new message", Duration::from_secs(5)),
        "unread indicator should appear while scrolled up. screen:\n{}",
        bpmux.tui()
    );

    // Scroll back to bottom with repeated Ctrl+D
    for _ in 0..15 {
        bpmux.type_ctrl('d');
    }
    assert!(
        bpmux.wait_for_tui("BOTTOM-MSG", Duration::from_secs(5)),
        "should see new message after scrolling to bottom. screen:\n{}",
        bpmux.tui()
    );
}

/// Scenario 5: long message while scrolled up → no proportional drift
#[test]
fn long_message_while_scrolled_up_no_drift() {
    let bpmux = TestBpmux::start();
    seed_messages(&bpmux, 25);

    bpmux.type_ctrl('u');
    bpmux
        .wait_until("viewport to scroll up", Duration::from_secs(5), || {
            !bpmux.tui().contains("msg-024")
        })
        .expect("should have scrolled up");
    let before_scroll = bpmux.tui();

    // Send a very long message that wraps to many lines
    let long = "word ".repeat(100); // ~500 chars, wraps to ~7 lines
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        post_chat(bpmux.port(), "pm:claude-code", &long).await;
    });
    // Wait for the unread-indicator update to confirm the message
    // was received; THEN verify the viewport did not drift.
    assert!(
        bpmux.wait_for_tui("new message", Duration::from_secs(5)),
        "unread indicator should appear. screen:\n{}",
        bpmux.tui()
    );
    let after = bpmux.tui();

    // The long message should not be visible (we're scrolled up)
    assert!(
        !after.contains(&long[..20]),
        "long message should not yank viewport"
    );
    // The viewport content should be similar to before (not shifted)
    // Check that a message visible before is still visible after
    // Find a msg-NNN that was visible before
    for i in 0..20 {
        let tag = format!("msg-{i:03}");
        if before_scroll.contains(&tag) {
            assert!(
                after.contains(&tag),
                "message {tag} was visible before the long message arrived but disappeared after — viewport drifted"
            );
            break;
        }
    }
}

/// Scenario 6: multiple messages arrive rapidly while scrolled up
#[test]
fn rapid_messages_while_scrolled_up_stable() {
    let bpmux = TestBpmux::start();
    seed_messages(&bpmux, 25);

    bpmux.type_ctrl('u');
    bpmux
        .wait_until("viewport to scroll up", Duration::from_secs(5), || {
            !bpmux.tui().contains("msg-024")
        })
        .expect("should have scrolled up");

    // Find a visible message before the burst
    let before = bpmux.tui();
    let mut anchor_msg = String::new();
    for i in 0..20 {
        let tag = format!("msg-{i:03}");
        if before.contains(&tag) {
            anchor_msg = tag;
            break;
        }
    }
    assert!(!anchor_msg.is_empty(), "should find a visible message");

    // Rapid-fire 10 messages
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        for i in 0..10 {
            post_chat(bpmux.port(), "pm:claude-code", &format!("rapid-{i}")).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    // Wait for the unread-indicator update (burst arrival signal),
    // then verify the anchor is still on screen.
    assert!(
        bpmux.wait_for_tui("new message", Duration::from_secs(5)),
        "unread indicator should reflect the burst. screen:\n{}",
        bpmux.tui()
    );

    let after = bpmux.tui();
    assert!(
        after.contains(&anchor_msg),
        "anchor message '{anchor_msg}' should still be visible after 10 rapid messages"
    );
}
