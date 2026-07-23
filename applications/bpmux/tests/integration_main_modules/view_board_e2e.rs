#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_board_renders_all_section_headers_on_empty_board() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");

        let text = view
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| view.to_string());
        for header in [
            "ICEBOX",
            "READY",
            "IN PROGRESS",
            "DESK CHECK",
            "ACCEPTANCE",
            "ACCEPTED",
        ] {
            assert!(
                text.contains(header),
                "view_board renders section header `{header}`; got:\n{text}"
            );
        }

        pm.close().await;
    });
}

#[test]
fn view_board_state_scope_renders_only_the_requested_column() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let view = pm
            .call_tool("view_board", serde_json::json!({ "state": "ready" }))
            .await
            .expect("scoped view_board should succeed");

        let text = view
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| view.to_string());

        assert!(
            text.contains("READY"),
            "scoped view_board(state=ready) renders the READY column; got:\n{text}"
        );
        for absent in [
            "ICEBOX",
            "IN PROGRESS",
            "DESK CHECK",
            "ACCEPTANCE",
            "ACCEPTED",
            "RETIRED",
        ] {
            assert!(
                !text.contains(absent),
                "scoped view_board(state=ready) must omit the `{absent}` column; got:\n{text}"
            );
        }

        pm.close().await;
    });
}
