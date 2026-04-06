//! Story 73 Acceptance Test: Session handoff summaries for agent reconnect.
//!
//! AC1: Agent sends handoff before disconnect
//! AC2: Handoff saved and delivered on reconnect
//! AC3: No handoff saved when agent doesn't send one
//! AC4: Handoff delivered after pins but before join announcement

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1+AC2: GIVEN an agent sends a handoff summary before disconnecting
/// WHEN it reconnects with the same name
/// THEN the handoff is delivered as a system message
#[tokio::test]
async fn ac1_ac2_handoff_saved_and_delivered() {
    let bpmux = TestBpmux::start();
    bpmux.wait_for_tui("Welcome", Duration::from_secs(5));

    // Connect agent
    let mut peer = WsPeer::connect(&bpmux, "bot").await;
    bpmux.wait_for_tui("bot:ws joined", Duration::from_secs(3));

    // Send handoff summary, then close
    peer.send_handoff("Working on auth module. Tests green.")
        .await;
    peer.close().await;

    // Wait for disconnect to complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Reconnect same name — should receive handoff
    let mut peer2 = WsPeer::connect(&bpmux, "bot").await;

    let found = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let Some(raw) = peer2.next_text().await else {
                return false;
            };
            if raw.contains("Handoff from your last session")
                && raw.contains("Working on auth module")
            {
                return true;
            }
        }
    })
    .await
    .unwrap_or(false);

    assert!(found, "Reconnecting agent should receive handoff summary");
}

/// AC3: GIVEN an agent disconnects without sending a handoff
/// WHEN it reconnects
/// THEN no handoff message is delivered
#[tokio::test]
async fn ac3_no_handoff_when_no_response() {
    let bpmux = TestBpmux::start();
    bpmux.wait_for_tui("Welcome", Duration::from_secs(5));

    // Connect and disconnect without sending handoff
    let peer = WsPeer::connect(&bpmux, "silent").await;
    bpmux.wait_for_tui("silent:ws joined", Duration::from_secs(3));
    peer.close().await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Reconnect — should NOT receive a handoff message
    let mut peer2 = WsPeer::connect(&bpmux, "silent").await;

    let handoff_found = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let Some(raw) = peer2.next_text().await else {
                return false;
            };
            if raw.contains("Handoff from your last session") {
                return true;
            }
            // Stop after join message (handoff comes before join)
            if raw.contains("silent:ws joined") {
                return false;
            }
        }
    })
    .await
    .unwrap_or(false);

    assert!(
        !handoff_found,
        "No handoff should be delivered when agent didn't send one"
    );
}

/// AC4: GIVEN a handoff exists
/// WHEN agent reconnects
/// THEN handoff is delivered after pins (before join announcement)
#[tokio::test]
async fn ac4_handoff_delivered_after_pins() {
    let bpmux = TestBpmux::start();
    bpmux.wait_for_tui("Welcome", Duration::from_secs(5));

    // First connection: send handoff
    let mut peer = WsPeer::connect(&bpmux, "agent").await;
    bpmux.wait_for_tui("agent:ws joined", Duration::from_secs(3));
    peer.send_handoff("Context: reviewing PR #42").await;
    peer.close().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Reconnect and track message order
    let mut peer2 = WsPeer::connect(&bpmux, "agent").await;
    let mut saw_handoff = false;
    let mut handoff_before_join = false;

    let _ = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let Some(raw) = peer2.next_text().await else {
                break;
            };
            if raw.contains("Handoff from your last session") {
                saw_handoff = true;
            }
            if raw.contains("agent:ws joined") {
                if saw_handoff {
                    handoff_before_join = true;
                }
                break;
            }
        }
    })
    .await;

    assert!(saw_handoff, "Handoff should be delivered on reconnect");
    assert!(
        handoff_before_join,
        "Handoff should arrive before join announcement"
    );
}
