//! Agent-side E2E test for `get_usage` MCP tool (#476 Phase 1
//! backfill — bridge-state-read archetype, file-backed). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — get_usage applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! **Substrate-quirk discovered**: `handle_get_usage`
//! (`bridge/tools/get_usage.rs:8`) reads from hardcoded
//! `~/.bpmux/usage.json` — does NOT honor `BPMUX_USAGE_PATH` env-var.
//! Write-side (`session::usage::default_usage_path()`) honors the env
//! var; read-side does not. Tests reading get_usage in CI-isolation
//! see whatever's in the developer's real `~/.bpmux/usage.json`. This
//! is a substrate-asymmetry that limits empty-state assertions; only
//! shape-correctness assertions are reliable here. Tracked for filing
//! per BUILDERS-vs-USERS lens (Chore-class — builder-substrate-quirk;
//! production agents see consistent path regardless).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn get_usage_returns_well_formed_payload() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("get_usage", serde_json::json!({}))
            .await
            .expect("get_usage should succeed");

        let parsed = match &result {
            serde_json::Value::Object(_) => result.clone(),
            serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(s)
                .expect("get_usage returns parseable JSON"),
            other => panic!("get_usage returned unexpected shape: {other:?}"),
        };
        assert!(
            parsed.is_object(),
            "get_usage returns a JSON object; got {parsed:?}"
        );

        dev.close().await;
    });
}
