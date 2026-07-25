#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_velocity_returns_metrics_json_for_empty_session() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("view_velocity", serde_json::json!({}))
            .await
            .expect("view_velocity should succeed");

        assert!(
            result
                .get("velocity_window_iterations")
                .is_some_and(|v| v.is_number())
                && result.get("iterations").is_some_and(|v| v.is_array())
                && result.get("rework").is_some_and(|v| v.is_object()),
            "view_velocity carries the window, per-iteration series, and rework rollup; got {result:?}"
        );

        pm.close().await;
    });
}
