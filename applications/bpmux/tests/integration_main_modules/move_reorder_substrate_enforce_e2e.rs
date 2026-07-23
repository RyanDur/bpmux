#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, ensure_current_iteration};
use std::time::Duration;

#[test]
fn dev_firing_reorder_returns_tool_retired_post() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] dev-reorder-tool-retired",
            "post-#1403 reorder_story is retired; dev MUST receive tool-retired (NOT workflow-role-not-allowed) — retire-gate precedes role-gate",
        )
        .await;

        let err = dev
            .call_tool(
                "reorder_story",
                serde_json::json!({
                    "id": target_id,
                    "position": 1,
                    "state": "ready",
                }),
            )
            .await
            .expect_err("dev firing retired reorder_story tool must error");

        assert!(
            err.contains("\"kind\":\"tool-retired\""),
            "structured kind cites tool-retired post-#1403 retire; got {err}"
        );
        assert!(
            err.contains("\"successor\":\"move_story\""),
            "rejection cites successor=move_story; got {err}"
        );
        assert!(
            err.contains("\"retired_axis\":\"reorder-tool\""),
            "rejection cites retired_axis=reorder-tool; got {err}"
        );
        assert!(
            !err.contains("workflow-role-not-allowed"),
            "retire-gate precedes role-gate post-#1403; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn designer_firing_reorder_returns_tool_retired_post() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] designer-reorder-tool-retired",
            "post-#1403 reorder_story is retired; designer MUST receive tool-retired",
        )
        .await;

        let err = designer
            .call_tool(
                "reorder_story",
                serde_json::json!({
                    "id": target_id,
                    "position": 1,
                    "state": "ready",
                }),
            )
            .await
            .expect_err("designer firing retired reorder_story tool must error");

        assert!(
            err.contains("\"kind\":\"tool-retired\""),
            "structured kind cites tool-retired post-#1403; got {err}"
        );
        assert!(
            err.contains("\"successor\":\"move_story\""),
            "rejection cites successor; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn dev_firing_move_with_position_returns_structured() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        ensure_current_iteration(&pm).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] dev-position-move-substrate-refusal",
            "reject if dev firing move with position isn't substrate-refused per PM-owns-queue-position canon",
        )
        .await;

        let err = dev
            .call_tool(
                "move_story",
                serde_json::json!({
                    "id": target_id,
                    "target_state": "ready",
                    "position": 1,
                    "rationale": "dev should not be able to set queue position per PM-owns-order canon",
                }),
            )
            .await
            .expect_err("dev should be substrate-refused at position-move axis");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind discriminator present; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"dev\""),
            "actor_role names firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"move-with-position\""),
            "attempted_transition names the gate; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}
