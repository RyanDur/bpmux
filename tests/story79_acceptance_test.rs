//! Story 79 Acceptance Test: Channel view shows only conversations
//! between the owner and the channel's participants.
//!
//! AC1: Owner selects "dev" channel → sees only owner↔dev messages
//! AC2: Messages between other agents are excluded from channel view
//! AC3: Owner sends a message while in a channel view → message appears
//! AC4: Viewing "broadcast" = only broadcast messages (unchanged)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN dev has sent a broadcast and a DM to the owner
/// WHEN the owner switches to the "dev" channel view
/// THEN only the DM is visible (broadcasts belong in broadcast view)
#[test]
fn ac1_channel_view_shows_only_owner_participant_messages() {
    let bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("broadcast from dev").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        dev.send_to("dm from dev", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    bpmux.type_line("/view dev");
    assert!(
        bpmux.wait_for_tui("View set to dev", Duration::from_secs(5)),
        "Should switch to dev view. Screen:\n{}",
        bpmux.tui()
    );

    let screen = bpmux.tui();
    assert!(
        screen.contains("dm from dev"),
        "DM from dev should be visible. Screen:\n{screen}"
    );
    assert!(
        !screen.contains("broadcast from dev"),
        "Broadcasts should not appear in channel views. Screen:\n{screen}"
    );
}

/// AC2: GIVEN dev has exchanged messages with both owner and pm
/// WHEN the owner switches to the "dev" channel view
/// THEN only owner↔dev messages appear, not dev↔pm messages
#[test]
fn ac2_channel_view_excludes_other_agents_messages() {
    let bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut pm = WsPeer::connect(&bpmux, "pm").await;

        dev.send_to("dev says hi", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        dev.send_to("dev to pm private", &["pm"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        pm.send_to("pm to dev reply", &["dev"]).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    bpmux.type_line("/view dev");
    assert!(
        bpmux.wait_for_tui("View set to dev", Duration::from_secs(5)),
        "Should switch to dev view. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("dev says hi", Duration::from_secs(5)),
        "DM from dev to owner should appear. Screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
    assert!(
        !screen.contains("dev to pm"),
        "dev↔pm messages should not appear in owner's dev view. Screen:\n{screen}"
    );
    assert!(
        !screen.contains("pm to dev"),
        "pm↔dev messages should not appear in owner's dev view. Screen:\n{screen}"
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
    assert!(
        bpmux.wait_for_tui("View set to dev", Duration::from_secs(5)),
        "view switch should land before owner types. screen:\n{}",
        bpmux.tui()
    );

    bpmux.type_line("owner reply in dev view");
    assert!(
        bpmux.wait_for_tui("owner reply in dev view", Duration::from_secs(5)),
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
    assert!(
        bpmux.wait_for_tui("View set to broadcast", Duration::from_secs(5)),
        "Should switch to broadcast view. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("broadcast msg", Duration::from_secs(5)),
        "Broadcast should appear. Screen:\n{}",
        bpmux.tui()
    );

    let screen = bpmux.tui();
    assert!(
        !screen.contains("directed msg"),
        "Directed message should NOT appear in broadcast view. Screen:\n{screen}"
    );
}
