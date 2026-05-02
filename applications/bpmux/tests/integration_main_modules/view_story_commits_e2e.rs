//! Agent-side E2E test for `view_story_commits` MCP tool (#476 Phase 1
//! backfill — read archetype, git-backed). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_story_commits applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! This handler runs `git log main --format=...` against the workspace
//! root (set via `BPMUX_ROOT` env-var in `pty.rs:104`). Tests run
//! against the bpmux dev workspace's actual git history, so number-
//! specific assertions reflect real commit messages.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_story_commits_returns_array_for_known_story() {
    // Per Ryan's "no git/GH in tests" directive (2026-05-02 02:39):
    // seed BPMUX_GIT_STUB_DIR with a fixture log containing a #476-
    // tagged commit so the test doesn't depend on the checkout's git
    // history (CI's shallow-clone fails this assertion otherwise).
    let stub_dir = tempfile::TempDir::new().expect("stub dir");
    let log_fixture = "abc1234567890abcdef1234567890abcdef1234\u{1f}abc1234\u{1f}Fix #476 substrate work\u{1f}dev\u{1f}2026-05-01T00:00:00+00:00\n";
    std::fs::write(stub_dir.path().join("log.txt"), log_fixture).expect("write fixture");
    let stub_dir_str = stub_dir.path().to_str().expect("stub dir path").to_string();
    let bpmux = TestBpmux::start_with_env("human", "BPMUX_GIT_STUB_DIR", &stub_dir_str);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        // handle_view_story_commits runs in the BRIDGE subprocess,
        // not the TestBpmux subprocess — the env var must reach the
        // bridge via connect_with_env.
        let dev = TestBridge::connect_with_env(
            &bpmux,
            "dev",
            &[("BPMUX_GIT_STUB_DIR", stub_dir_str.as_str())],
        )
        .await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("view_story_commits", serde_json::json!({"number": 476}))
            .await
            .expect("view_story_commits should succeed");

        let parsed = match &result {
            serde_json::Value::Array(_) => result.clone(),
            serde_json::Value::String(s) => {
                serde_json::from_str::<serde_json::Value>(s).expect("returns parseable JSON")
            }
            other => panic!("view_story_commits returned unexpected shape: {other:?}"),
        };
        assert!(
            parsed.is_array(),
            "view_story_commits returns a JSON array; got {parsed:?}"
        );
        let arr = parsed.as_array().expect("array");
        assert!(
            !arr.is_empty(),
            "#476 has commits in the workspace history; expected non-empty"
        );
        let first = &arr[0];
        assert!(first["sha"].is_string(), "commit has sha; got {first:?}");
        assert!(
            first["subject"].is_string(),
            "commit has subject; got {first:?}"
        );
        assert!(
            first["author"].is_string(),
            "commit has author; got {first:?}"
        );

        dev.close().await;
    });
}

#[test]
fn view_story_commits_returns_empty_for_unknown_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool(
                "view_story_commits",
                serde_json::json!({"number": 999999999}),
            )
            .await
            .expect("view_story_commits should succeed");

        let parsed = match &result {
            serde_json::Value::Array(_) => result.clone(),
            serde_json::Value::String(s) => {
                serde_json::from_str::<serde_json::Value>(s).expect("returns parseable JSON")
            }
            other => panic!("view_story_commits returned unexpected shape: {other:?}"),
        };
        let arr = parsed.as_array().expect("array");
        assert!(
            arr.is_empty(),
            "no commits reference #999999999; expected empty array; got {arr:?}"
        );

        dev.close().await;
    });
}

#[test]
fn view_story_commits_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("view_story_commits", serde_json::json!({}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("number"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}
