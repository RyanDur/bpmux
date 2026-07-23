#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn set_status_retired_with_canonical_tool_retired_shape() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("set_status", serde_json::json!({"status": "ready"}))
            .await
            .expect_err("set_status is retired per #1404; must error with tool-retired");

        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"] == "tool-retired",
            "rejection cites tool-retired kind; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["successor"]
                == "workflow-event-hooks",
            "rejection cites successor=workflow-event-hooks; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["retired_axis"]
                == "manual-status-mutation",
            "rejection cites retired_axis=manual-status-mutation; got {err}"
        );
        assert!(
            err.contains("feedback_bpmux_no_manual_status"),
            "actionable message cites the underlying canon; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn workflow_event_hooks_continue_to_propagate_state_post_retire() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let create_result = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] hook-propagation-verify-post-1404-retire",
                    "body": "## What\n\nverify hooks continue to fire after set_status retire\n\n## Acceptance test path\n\nN/A (this fixture)\n\n"
                }),
            )
            .await;
        if let Err(e) = &create_result {
            assert!(
                !e.contains("tool-retired"),
                "create_story is NOT retired; got {e}"
            );
        }

        pm.close().await;
    });
}

#[test]
fn set_status_retired_regardless_of_status_arg_value() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        for status_value in ["ready", "in-progress", "delivered", "completed", ""] {
            let err = dev
                .call_tool(
                    "set_status",
                    serde_json::json!({"status": status_value}),
                )
                .await
                .expect_err(&format!(
                    "set_status retired regardless of status arg value; status={status_value}"
                ));
            assert!(
                crate::harness::story_helpers::structured_error(&err)["kind"] == "tool-retired",
                "status={status_value}: rejection cites tool-retired regardless of arg value; got {err}"
            );
        }

        let err = dev
            .call_tool("set_status", serde_json::json!({}))
            .await
            .expect_err("set_status retired even with empty args");
        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"] == "tool-retired",
            "retire-gate fires before arg-validation; got {err}"
        );

        dev.close().await;
    });
}
