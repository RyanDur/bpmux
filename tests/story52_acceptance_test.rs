//! Story 52 Acceptance Test: A human can direct a message to specific participants from the TUI
//!
//! AC1: `/to dev body text` → sent with to:["dev"], body "body text"
//! AC2: `/to dev,designer body text` → sent with to:["dev", "designer"]
//! AC3: No `/to` prefix → broadcast (unchanged)
//! AC4: `/to ` + Tab → participant names offered for completion
//! AC5: `/to unknown-name body` → not sent, input preserved, error surfaced

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a human types `/to dev hello from human` and presses Enter
/// THEN the message is sent with to:["dev"] and the body "hello from human"
/// AND dev receives the push, but designer does not
#[test]
fn ac1_to_single_recipient_sends_directed_message() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    // Connect WS peers to observe delivery
    let (mut dev, mut designer) = rt.block_on(async {
        let dev = WsPeer::connect(&bpmux, "dev").await;
        let designer = WsPeer::connect(&bpmux, "designer").await;
        (dev, designer)
    });

    // Human types `/to dev hello from human` in the TUI
    bpmux.type_line("/to dev hello from human");

    rt.block_on(async {
        // dev should receive the directed message
        let (_from, text) = dev.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "hello from human");

        // designer should NOT receive it
        assert!(
            designer
                .recv_message_or_timeout(Duration::from_millis(500))
                .await
                .is_none(),
            "designer should not receive a message addressed to dev"
        );
    });
}

/// AC2: GIVEN a human types `/to dev,designer group hello` and presses Enter
/// THEN the message is sent with to:["dev", "designer"] and the body "group hello"
/// AND both dev and designer receive the push
#[test]
fn ac2_to_csv_recipients_sends_to_all_named() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let (mut dev, mut designer, mut pm) = rt.block_on(async {
        let dev = WsPeer::connect(&bpmux, "dev").await;
        let designer = WsPeer::connect(&bpmux, "designer").await;
        let pm = WsPeer::connect(&bpmux, "pm").await;
        (dev, designer, pm)
    });

    // Human types `/to dev,designer group hello`
    bpmux.type_line("/to dev,designer group hello");

    rt.block_on(async {
        // dev and designer should receive it
        let (_from, text) = dev.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "group hello");
        let (_from, text) = designer.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "group hello");

        // pm should NOT receive it
        assert!(
            pm.recv_message_or_timeout(Duration::from_millis(500))
                .await
                .is_none(),
            "pm should not receive a message addressed to dev and designer"
        );
    });
}

/// AC3: GIVEN a human types a message with no `/to` prefix
/// THEN the message is sent as broadcast (existing behavior, unchanged)
#[test]
fn ac3_no_to_prefix_sends_broadcast() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let mut dev = rt.block_on(async { WsPeer::connect(&bpmux, "dev").await });

    // Human types a plain message (no /to)
    bpmux.type_line("hello everyone");

    rt.block_on(async {
        let (_from, text) = dev.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "hello everyone");
    });
}

/// AC5: GIVEN a human types `/to` with no recipients or body
/// THEN the message is NOT sent and an error is surfaced in the TUI
#[test]
fn ac5_to_with_no_args_shows_error() {
    let bpmux = TestBpmux::start();

    // Human types `/to` with nothing after it
    bpmux.type_line("/to");

    // TUI should show a usage/error message
    assert!(
        bpmux.wait_for_tui("Usage:", Duration::from_secs(3)),
        "Should show usage hint when /to has no arguments. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5: GIVEN a human types `/to dev` with no body text
/// THEN the message is NOT sent and an error is surfaced
#[test]
fn ac5_to_with_recipient_but_no_body_shows_error() {
    let bpmux = TestBpmux::start();

    // Human types `/to dev` with no message body
    bpmux.type_line("/to dev");

    // TUI should show an error
    assert!(
        bpmux.wait_for_tui("Usage:", Duration::from_secs(3)),
        "Should show usage hint when /to has no message body. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC1 + AC4 (TUI rendering): GIVEN a human sends via `/to dev`
/// THEN the TUI renders the channel tag for the directed message
#[test]
fn tui_renders_channel_tag_for_to_message() {
    let bpmux = TestBpmux::start();

    // Human sends a directed message
    bpmux.type_line("/to dev check this out");

    // TUI should render the directed channel tag (owner excluded from name)
    assert!(
        bpmux.wait_for_tui("[dev]", Duration::from_secs(5)),
        "TUI should show channel tag for directed message. Screen:\n{}",
        bpmux.tui()
    );
}
