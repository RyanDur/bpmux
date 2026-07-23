#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::ensure_current_iteration;
use std::time::Duration;

#[test]
fn reorder_story_retired_with_canonical_tool_retired_shape() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "reorder_story",
                serde_json::json!({
                    "id": 100,
                    "position": 1,
                    "state": "ready"
                }),
            )
            .await
            .expect_err("reorder_story is retired per #1403; must error with tool-retired");

        assert!(
            err.contains("\"kind\":\"tool-retired\""),
            "rejection cites tool-retired kind; got {err}"
        );
        assert!(
            err.contains("\"successor\":\"move_story\""),
            "rejection cites successor; got {err}"
        );
        assert!(
            err.contains("\"retired_axis\":\"reorder-tool\""),
            "rejection cites retired_axis; got {err}"
        );
        assert!(
            err.contains("move_story") && err.contains("target_state") && err.contains("position"),
            "actionable message guides caller to successor + args; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn move_story_with_same_target_state_preserves_reorder_semantic() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        ensure_current_iteration(&pm).await;

        let result = pm
            .call_tool(
                "move_story",
                serde_json::json!({
                    "id": 999_999_999,
                    "target_state": "ready",
                    "position": 1,
                    "rationale": "reorder per priority refresh (#1403 retire-replacement)"
                }),
            )
            .await;
        if let Err(e) = &result {
            assert!(
                !e.contains("tool-retired"),
                "move_story is NOT retired; must not surface tool-retired; got {e}"
            );
        }

        pm.close().await;
    });
}
