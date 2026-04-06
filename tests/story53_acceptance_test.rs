//! Story 53 Acceptance Test: A human can see which channels have activity
//! from the TUI status bar.
//!
//! AC1: Status bar shows `[all]` view label and channel count
//! AC2: New directed message increments channel count
//! AC3: Broadcast-only session shows `1 channel`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC3: GIVEN only broadcast traffic exists
/// WHEN the human looks at the status bar
/// THEN it shows `[all] · 1 channel`
#[test]
fn ac3_broadcast_only_shows_one_channel() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("hello everyone").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("[all]", Duration::from_secs(5)),
        "Status bar should show [all] view label. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("1 channel", Duration::from_secs(5)),
        "Status bar should show 1 channel for broadcast-only. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC1: GIVEN a session with broadcast and directed traffic
/// WHEN the human looks at the status bar
/// THEN the status bar shows `[all]` and a channel count
#[test]
fn ac1_status_bar_shows_view_label_and_channel_count() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("broadcast msg").await;
        dev.send_to("hey human", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("[all]", Duration::from_secs(5)),
        "Status bar should show [all]. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("2 channels", Duration::from_secs(5)),
        "Status bar should show 2 channels (broadcast + directed). Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN a new directed message arrives on a new channel
/// WHEN the message is logged
/// THEN the channel count increments
#[test]
fn ac2_new_directed_message_increments_channel_count() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("broadcast").await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    assert!(
        bpmux.wait_for_tui("1 channel", Duration::from_secs(5)),
        "Should start with 1 channel. Screen:\n{}",
        bpmux.tui()
    );

    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_to("private to human", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    assert!(
        bpmux.wait_for_tui("2 channels", Duration::from_secs(5)),
        "Should show 2 channels after directed message. Screen:\n{}",
        bpmux.tui()
    );

    rt.block_on(async {
        let mut pm = WsPeer::connect(&bpmux, "pm").await;
        pm.send_to("dev and human", &["dev", "human"]).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    assert!(
        bpmux.wait_for_tui("3 channels", Duration::from_secs(5)),
        "Should show 3 channels after new group. Screen:\n{}",
        bpmux.tui()
    );
}
