//! Story 55 Acceptance Test — REVISED
//!
//! Ryan rejected the broadcast-default behavior during acceptance.
//! Default view is now [all]. Opt-in filtering happens via the
//! clickable sidebar (#223) — the original /view slash command
//! was removed in #231.
//!
//! AC1: Session starts in [all] view (revised from original)
//! AC2: Status bar shows channel count in [all] view
//! AC3: opt-in broadcast filter — coverage moved to click-path
//!      regression in src/terminal/app_tests.rs
//!      (clicking_channel_sets_current_view_and_resets_filter_and_scroll).
//!      The PTY-level equivalent using sidebar mouse clicks is a
//!      follow-up if Ryan wants end-to-end coverage of the opt-in.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

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
