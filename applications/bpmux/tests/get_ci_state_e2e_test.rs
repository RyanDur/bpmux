//! Agent-side E2E test for `get_ci_state` MCP tool (#476 Phase 1
//! backfill — bridge-state-read archetype, optional-API variant).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — get_ci_state applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Bridge constructs GitHubApi only when GH env-vars are set; without
//! them, the API is None and `get_ci_state` returns the empty
//! report shape (no error). Tests run without GH env so this exercises
//! the empty-state path.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn get_ci_state_returns_well_formed_report_without_gh_env() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("get_ci_state", serde_json::json!({"refresh": false}))
            .await
            .expect("get_ci_state should succeed without GH env (empty-report path)");

        let parsed = match &result {
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => result.clone(),
            serde_json::Value::String(s) => {
                serde_json::from_str::<serde_json::Value>(s).expect("returns parseable JSON")
            }
            other => panic!("get_ci_state returned unexpected shape: {other:?}"),
        };
        assert!(
            parsed.is_object() || parsed.is_array(),
            "get_ci_state returns structured JSON; got {parsed:?}"
        );

        dev.close().await;
    });
}
