//! Story 55 Acceptance Test — REVISED
//!
//! Ryan rejected the broadcast-default behavior during acceptance.
//! Default view is now [all]. The /view command still works for
//! filtering when the user wants it.
//!
//! AC1: Session starts in [all] view (revised from original)
//! AC2: Status bar shows channel count in [all] view
//! AC3: /view broadcast still works as an opt-in filter

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// REVISED AC1: Session starts in [all] view
#[test]
fn ac1_session_starts_in_all_view() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("public hello").await;
        dev.send_to("secret dm", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    // Both messages should be visible in default [all] view
    assert!(
        bpmux.wait_for_tui("public hello", Duration::from_secs(5)),
        "Broadcast should be visible. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("secret dm", Duration::from_secs(5)),
        "Directed message should also be visible in default all view. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: Status bar shows [all] and channel count
#[test]
fn ac2_status_bar_shows_all_and_channels() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("broadcast msg").await;
        dev.send_to("dm", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("[all]", Duration::from_secs(5)),
        "Status bar should show [all]. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("2 channels", Duration::from_secs(5)),
        "Status bar should show 2 channels. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: /view broadcast works as opt-in filter
#[test]
fn ac3_view_broadcast_is_opt_in() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("public hello").await;
        dev.send_to("secret dm", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    // Both visible initially
    assert!(
        bpmux.wait_for_tui("secret dm", Duration::from_secs(5)),
        "DM should be visible in default view. Screen:\n{}",
        bpmux.tui()
    );
    // Opt into broadcast filter
    bpmux.type_line("/view broadcast");
    assert!(
        bpmux.wait_for_tui("View set to broadcast", Duration::from_secs(5)),
        "Should confirm broadcast switch. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        !bpmux.wait_for_tui("secret dm", Duration::from_secs(1)),
        "DM should be hidden in broadcast view. Screen:\n{}",
        bpmux.tui()
    );
}
