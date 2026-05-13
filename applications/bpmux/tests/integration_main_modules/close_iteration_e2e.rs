//! Agent-side E2E test for `close_iteration` MCP tool (#476 Phase 1
//! backfill — signal-broadcast variant, pm-only). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — close_iteration applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Fires the iteration-close externalize-prompt broadcast (#282); a
//! "reminder, not a barrier" — agents may flush unsaved findings.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn close_iteration_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("close_iteration", serde_json::json!({}))
            .await
            .expect_err("dev should be rejected by close_iteration pm-lane gate");

        assert!(
            err.contains("pm") || err.contains("only") || err.contains("restricted"),
            "rejection cites pm-lane discipline; got {err}"
        );

        dev.close().await;
    });
}
