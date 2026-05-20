#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn set_status_returns_tool_retired_post_1404() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        for role in ["dev", "pm", "designer"] {
            let bridge = TestBridge::connect(&bpmux, role).await;
            tokio::time::sleep(Duration::from_millis(500)).await;

            let err = bridge
                .call_tool(
                    "set_status",
                    serde_json::json!({"status": "STATUS_MARKER_SET_S1"}),
                )
                .await
                .expect_err(&format!(
                    "{role}: set_status is retired; must surface tool-retired"
                ));

            assert!(
                err.contains("\"kind\":\"tool-retired\""),
                "{role}: structured kind cites tool-retired; got {err}"
            );
            assert!(
                err.contains("\"successor\":\"workflow-event-hooks\""),
                "{role}: rejection cites successor=workflow-event-hooks; got {err}"
            );
            assert!(
                err.contains("\"retired_axis\":\"manual-status-mutation\""),
                "{role}: rejection cites retired_axis=manual-status-mutation; got {err}"
            );

            bridge.close().await;
        }
    });
}
