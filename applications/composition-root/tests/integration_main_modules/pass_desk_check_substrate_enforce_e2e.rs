#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn dev_firing_pass_desk_check_returns_workflow_role_not_allowed() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] dev-pass-desk-check-substrate-refusal",
            "reject if dev firing pass_desk_check isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = dev
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"id": target_id, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect_err("dev should be substrate-refused");

        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"] == "workflow-role-not-allowed",
            "rejection cites structured kind; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["actor_role"] == "dev",
            "rejection cites firing role; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["attempted_transition"] == "pass-desk-check",
            "rejection cites the attempted transition; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn designer_firing_pass_desk_check_without_broadcast_returns_structured() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] designer-pass-without-evidence",
            "reject if designer firing pass_desk_check without recent substrate-broadcast succeeds",
        )
        .await;

        let err = designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"id": target_id, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect_err("pass_desk_check without designer-evidence should be refused");

        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"]
                == "workflow-gate-mismatch",
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"missing_evidence\":[\"designer-verdict-missing\"]"),
            "rejection cites missing designer-verdict evidence; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn designer_firing_pass_desk_check_with_broadcast_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] designer-pass-with-evidence",
            "reject if designer firing pass_desk_check with recent substrate-broadcast doesn't 200",
        )
        .await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_id": target_id,
                    "verdict": "verified",
                    "substrate_grounding": "rg + git show fragments cited at src/foo.rs:42"
                }),
            )
            .await
            .expect("designer submit_verdict");

        let result = designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"id": target_id, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect("pass_desk_check with designer-evidence should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_id}")),
            "happy-path message names the story; got {msg}"
        );

        pm.close().await;
        designer.close().await;
    });
}
