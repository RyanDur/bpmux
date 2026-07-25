#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn get_sync_status_returns_baseline_shape_post_boot() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("get_sync_status", serde_json::json!({}))
            .await
            .expect("get_sync_status should succeed");

        assert!(
            result.get("last_ingest_at").is_some(),
            "snapshot exposes last_ingest_at; got {result:?}"
        );
        assert!(
            result.get("last_push_at").is_some(),
            "snapshot exposes last_push_at; got {result:?}"
        );
        assert_eq!(
            result["pending_events"], 0,
            "post-boot snapshot has zero pending events; got {result:?}"
        );
        assert!(
            result.get("last_error").is_some(),
            "snapshot exposes last_error (null when clean); got {result:?}"
        );

        pm.close().await;
    });
}
