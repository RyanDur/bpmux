//! Story 79 Acceptance Test: Channel view shows messages by participant
//! involvement, not exact addressing.
//!
//! AC1: Owner selects "dev" channel → sees broadcasts from dev
//! AC2: Owner selects "dev" channel → sees all messages from/to dev
//! AC3: Owner sends a message while in a channel view → message appears
//! AC4: Viewing "broadcast" = only broadcast messages (unchanged)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN dev has sent both a broadcast and a directed message
/// WHEN the owner switches to the "dev" channel view
/// THEN the broadcast from dev is also visible (not just the DM)
#[test]
fn ac1_broadcasts_visible_in_participant_view() {
    let bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        // Broadcast creates a "broadcast" channel entry
        dev.send("broadcast from dev").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        // DM to owner creates a "dev" channel entry
        dev.send_to("dm from dev", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Switch to "dev" channel view
    bpmux.type_line("/view dev");
    std::thread::sleep(Duration::from_millis(500));

    // Verify the view changed
    assert!(
        bpmux.wait_for_tui("View set to dev", Duration::from_secs(3)),
        "Should switch to dev view. Screen:\n{}",
        bpmux.tui()
    );

    // The broadcast from dev should be visible in the dev channel view
    let screen = bpmux.tui();
    assert!(
        screen.contains("broadcast from dev"),
        "Broadcast from dev should be visible in dev channel view. Screen:\n{screen}"
    );
}

/// AC2: GIVEN dev has exchanged directed messages with pm
/// WHEN the owner switches to the "dev" channel view
/// THEN messages both from and to dev appear
#[test]
fn ac2_directed_messages_visible_in_participant_view() {
    let bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut pm = WsPeer::connect(&bpmux, "pm").await;

        // DM from dev to owner — creates "dev" channel
        dev.send_to("dev says hi", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        // DM from dev to pm — creates "dev+pm" channel
        dev.send_to("dev to pm private", &["pm"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        // DM from pm to dev — also involves dev
        pm.send_to("pm to dev reply", &["dev"]).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Switch to "dev" view — should see messages involving dev
    bpmux.type_line("/view dev");
    std::thread::sleep(Duration::from_millis(500));

    let screen = bpmux.tui();
    assert!(
        screen.contains("dev says hi"),
        "DM from dev to owner should appear. Screen:\n{screen}"
    );
    assert!(
        screen.contains("dev to pm"),
        "DM from dev to pm should appear in dev view. Screen:\n{screen}"
    );
    assert!(
        screen.contains("pm to dev"),
        "DM from pm to dev should appear in dev view. Screen:\n{screen}"
    );
}

/// AC3: GIVEN the owner is in a channel view
/// WHEN they send a message
/// THEN the message appears in that view
#[test]
fn ac3_owner_message_visible_in_channel_view() {
    let bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_to("ping from dev", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    bpmux.type_line("/view dev");
    std::thread::sleep(Duration::from_millis(300));

    bpmux.type_line("owner reply in dev view");
    std::thread::sleep(Duration::from_millis(500));

    assert!(
        bpmux.wait_for_tui("owner reply in dev view", Duration::from_secs(3)),
        "Owner's message should appear in the channel view. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN the owner selects "broadcast" view
/// WHEN directed messages exist
/// THEN only broadcast messages appear
#[test]
fn ac4_broadcast_view_unchanged() {
    let bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("broadcast msg").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        dev.send_to("directed msg", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    bpmux.type_line("/view broadcast");
    std::thread::sleep(Duration::from_millis(500));

    let screen = bpmux.tui();
    assert!(
        screen.contains("broadcast msg"),
        "Broadcast should appear. Screen:\n{screen}"
    );
    assert!(
        !screen.contains("directed msg"),
        "Directed message should NOT appear in broadcast view. Screen:\n{screen}"
    );
}
