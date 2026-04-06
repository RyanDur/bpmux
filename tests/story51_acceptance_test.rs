//! Story 51 Acceptance Test: A participant can address a message to specific recipients
//!
//! AC1: Single recipient → only that recipient gets push, message in log
//! AC2: Multiple recipients → each gets push, non-recipients don't
//! AC3: Broadcast (no addressee or to:["all"]) → everyone gets push
//! AC4: TUI renders channel tag prefix on every message

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a participant sends a message with one named recipient
/// THEN only that named recipient receives a push delivery
/// AND the message appears in the shared log
#[test]
fn ac1_single_recipient_gets_push_others_do_not() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut pm = WsPeer::connect(&bpmux, "pm").await;
        let mut designer = WsPeer::connect(&bpmux, "designer").await;

        // dev sends a message addressed only to pm
        dev.send_to("secret for pm", &["pm"]).await;

        // pm should receive it
        let (_from, text) = pm.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "secret for pm");

        // designer should NOT receive it
        assert!(
            designer
                .recv_message_or_timeout(Duration::from_millis(500))
                .await
                .is_none(),
            "designer should not receive a message addressed to pm"
        );
    });

    // The message should appear in the TUI (log captures everything)
    assert!(
        bpmux.wait_for_tui("secret for pm", Duration::from_secs(5)),
        "Addressed message should appear in TUI (log shows all). Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN a participant sends a message with multiple named recipients
/// THEN each named recipient receives push
/// AND no other agents receive push
#[test]
fn ac2_multiple_recipients_get_push_others_do_not() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut pm = WsPeer::connect(&bpmux, "pm").await;
        let mut designer = WsPeer::connect(&bpmux, "designer").await;
        let mut ryan = WsPeer::connect(&bpmux, "ryan").await;

        // dev sends to pm and designer
        dev.send_to("group msg", &["pm", "designer"]).await;

        // pm and designer should receive it
        let (_from, text) = pm.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "group msg");
        let (_from, text) = designer.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "group msg");

        // ryan should NOT receive it
        assert!(
            ryan.recv_message_or_timeout(Duration::from_millis(500))
                .await
                .is_none(),
            "ryan should not receive a message addressed to pm and designer"
        );
    });
}

/// AC3: GIVEN a participant sends a broadcast (no addressee)
/// THEN every connected participant receives push
#[test]
fn ac3_broadcast_no_addressee_reaches_everyone() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut pm = WsPeer::connect(&bpmux, "pm").await;

        // dev sends a broadcast (no addressee)
        dev.send("broadcast msg").await;

        // pm should receive it
        let (_from, text) = pm.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "broadcast msg");
    });
}

/// AC3 variant: to:["all"] is also broadcast
#[test]
fn ac3_to_all_is_broadcast() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut pm = WsPeer::connect(&bpmux, "pm").await;

        // dev sends with to:["all"]
        dev.send_to("all-broadcast", &["all"]).await;

        // pm should receive it
        let (_from, text) = pm.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "all-broadcast");
    });
}

/// AC4: GIVEN a message is rendered in the TUI
/// THEN it is prefixed with a channel tag identifying the addressing
#[test]
fn ac4_tui_renders_channel_tags() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;

        // Broadcast message
        dev.send("hello all").await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Addressed DM to human (the TUI owner)
        dev.send_to("hey human", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    });

    // TUI should show [broadcast] tag
    assert!(
        bpmux.wait_for_tui("[broadcast]", Duration::from_secs(5)),
        "Broadcast messages should show [broadcast] tag. Screen:\n{}",
        bpmux.tui()
    );

    // TUI should show DM tag (owner excluded from channel name)
    assert!(
        bpmux.wait_for_tui("[dev]", Duration::from_secs(5)),
        "DM messages should show [dev] tag. Screen:\n{}",
        bpmux.tui()
    );
}
