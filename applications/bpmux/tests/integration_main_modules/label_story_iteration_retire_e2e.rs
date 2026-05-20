#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn label_story_rejects_iteration_label_with_tool_retired_kind() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "label_story",
                serde_json::json!({
                    "number": 100,
                    "label": "iteration-Telescopium.100",
                    "operation": "add"
                }),
            )
            .await
            .expect_err("iteration-axis label must be rejected post-#1402 retire");

        assert!(
            err.contains("\"kind\":\"tool-retired\""),
            "rejection cites tool-retired kind; got {err}"
        );
        assert!(
            err.contains("\"successor\":\"add_story_to_iteration\""),
            "rejection cites successor; got {err}"
        );
        assert!(
            err.contains("\"retired_axis\":\"iteration-label\""),
            "rejection cites retired_axis; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn label_story_accepts_non_iteration_labels_unchanged() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool(
                "label_story",
                serde_json::json!({
                    "number": 100,
                    "label": "blocked",
                    "operation": "add"
                }),
            )
            .await;
        if let Err(e) = &result {
            assert!(
                !e.contains("tool-retired"),
                "non-iteration label must NOT trip retire-gate; got {e}"
            );
        }

        pm.close().await;
    });
}
