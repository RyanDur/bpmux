//! Story 70 Acceptance Test: A participant can pin decisions so reconnecting
//! agents have theory context.
//!
//! AC1: /pin adds a visible pin
//! AC2: Pins delivered to connecting agents
//! AC3: /pins lists all pins
//! AC4: /unpin removes a pin
//! AC5: Max 5 pins enforced

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a participant types `/pin <text>`
/// WHEN the command executes
/// THEN the pin appears in the TUI
#[test]
fn ac1_pin_adds_visible_pin() {
    let bpmux = TestBpmux::start();

    bpmux.type_line("/pin channels-as-views because filter-on-render");
    assert!(
        bpmux.wait_for_tui("Pinned #1", Duration::from_secs(5)),
        "Should confirm pin. Screen:\n{}",
        bpmux.tui()
    );
    // Pin bar should render
    assert!(
        bpmux.wait_for_tui("channels-as-views", Duration::from_secs(5)),
        "Pin content should be visible. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN pins exist
/// WHEN a new agent connects
/// THEN the pinned items are delivered before message history
#[test]
fn ac2_pins_delivered_on_connect() {
    let bpmux = TestBpmux::start();

    // Pin something first
    bpmux.type_line("/pin log-as-primitive is the core invariant");
    assert!(
        bpmux.wait_for_tui("Pinned #1", Duration::from_secs(5)),
        "Should confirm pin. Screen:\n{}",
        bpmux.tui()
    );

    // New agent connects — pins arrive as system message right after welcome
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let found_pin = rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        // Read the first few frames looking for pins (they come before join announcement)
        for _ in 0..5 {
            if let Some(raw) = tokio::time::timeout(Duration::from_secs(3), dev.next_text())
                .await
                .ok()
                .flatten()
            {
                if raw.contains("log-as-primitive") {
                    return true;
                }
            }
        }
        false
    });
    assert!(
        found_pin,
        "Connecting agent should receive pinned items"
    );
}

/// AC3: GIVEN pins exist
/// WHEN a participant types `/pins`
/// THEN all pins are listed
#[test]
fn ac3_pins_lists_all() {
    let bpmux = TestBpmux::start();

    bpmux.type_line("/pin theory one");
    assert!(
        bpmux.wait_for_tui("Pinned #1", Duration::from_secs(5)),
        "Screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_line("/pin theory two");
    assert!(
        bpmux.wait_for_tui("Pinned #2", Duration::from_secs(5)),
        "Screen:\n{}",
        bpmux.tui()
    );

    bpmux.type_line("/pins");
    assert!(
        bpmux.wait_for_tui("1. theory one", Duration::from_secs(5)),
        "Should list pin 1. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("2. theory two", Duration::from_secs(5)),
        "Should list pin 2. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN a pin exists
/// WHEN a participant types `/unpin 1`
/// THEN the pin is removed
#[test]
fn ac4_unpin_removes_pin() {
    let bpmux = TestBpmux::start();

    bpmux.type_line("/pin removable item");
    assert!(
        bpmux.wait_for_tui("Pinned #1", Duration::from_secs(5)),
        "Screen:\n{}",
        bpmux.tui()
    );

    bpmux.type_line("/unpin 1");
    assert!(
        bpmux.wait_for_tui("Unpinned #1", Duration::from_secs(5)),
        "Should confirm unpin. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5: GIVEN 5 pins exist
/// WHEN a participant tries to add a 6th
/// THEN a system message says max 5
#[test]
fn ac5_max_five_pins() {
    let bpmux = TestBpmux::start();

    for i in 1..=5 {
        bpmux.type_line(&format!("/pin decision {i}"));
        assert!(
            bpmux.wait_for_tui(&format!("Pinned #{i}"), Duration::from_secs(5)),
            "Should confirm pin {i}. Screen:\n{}",
            bpmux.tui()
        );
    }

    bpmux.type_line("/pin one too many");
    assert!(
        bpmux.wait_for_tui("max 5 pins", Duration::from_secs(5)),
        "Should reject 6th pin. Screen:\n{}",
        bpmux.tui()
    );
}
