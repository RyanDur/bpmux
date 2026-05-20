#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn reorder_story_returns_tool_retired_for_any_role() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        for role in ["pm", "dev"] {
            let bridge = TestBridge::connect(&bpmux, role).await;
            tokio::time::sleep(Duration::from_millis(500)).await;

            let err = bridge
                .call_tool(
                    "reorder_story",
                    serde_json::json!({
                        "number": 100,
                        "position": 0,
                        "state": "ready"
                    }),
                )
                .await
                .expect_err(&format!(
                    "{role}: reorder_story is retired; must surface tool-retired"
                ));

            assert!(
                err.contains("\"kind\":\"tool-retired\""),
                "{role}: structured kind cites tool-retired; got {err}"
            );
            assert!(
                err.contains("\"successor\":\"move_story\""),
                "{role}: rejection cites successor=move_story; got {err}"
            );
            assert!(
                err.contains("\"retired_axis\":\"reorder-tool\""),
                "{role}: rejection cites retired_axis=reorder-tool; got {err}"
            );

            bridge.close().await;
        }
    });
}
