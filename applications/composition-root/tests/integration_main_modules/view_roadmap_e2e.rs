#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_roadmap_returns_content_string() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("view_roadmap", serde_json::json!({}))
            .await
            .expect("view_roadmap should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("NOW") || text.contains("NEXT") || text.contains("outcome"),
            "view_roadmap returns non-empty content (or honest-error string); got `{text}`"
        );

        pm.close().await;
    });
}
