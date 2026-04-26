//! Agent-side E2E test for `save_bookmark` + `get_bookmark` MCP tool
//! pair (#476 Phase 1 backfill — memory archetype, paired write/read).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — bookmark applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn save_bookmark_returns_confirmation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool(
                "save_bookmark",
                serde_json::json!({
                    "current_task": "e2e-bookmark-test-task",
                    "next_step": "verify roundtrip via get_bookmark"
                }),
            )
            .await
            .expect("save_bookmark should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("bookmark saved"),
            "happy-path message confirms persistence; got {msg}"
        );

        dev.close().await;
    });
}

#[test]
fn save_bookmark_get_bookmark_roundtrip_preserves_fields_verbatim() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let task_marker = "BOOKMARK_TASK_MARKER_R1";
        let next_marker = "BOOKMARK_NEXT_MARKER_R1";
        let notes_marker = "BOOKMARK_NOTES_MARKER_R1";

        dev.call_tool(
            "save_bookmark",
            serde_json::json!({
                "current_task": task_marker,
                "next_step": next_marker,
                "notes": notes_marker,
                "decisions": ["dec-1", "dec-2"],
                "open_questions": ["q-1"]
            }),
        )
        .await
        .expect("save_bookmark should succeed");

        let bookmark = dev
            .call_tool("get_bookmark", serde_json::json!({}))
            .await
            .expect("get_bookmark should succeed");

        let bookmark_text = bookmark
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| bookmark.to_string());
        assert!(
            bookmark_text.contains(task_marker),
            "current_task preserved verbatim; got:\n{bookmark_text}"
        );
        assert!(
            bookmark_text.contains(next_marker),
            "next_step preserved verbatim; got:\n{bookmark_text}"
        );
        assert!(
            bookmark_text.contains(notes_marker),
            "notes preserved verbatim; got:\n{bookmark_text}"
        );
        assert!(
            bookmark_text.contains("dec-1") && bookmark_text.contains("dec-2"),
            "decisions array preserved; got:\n{bookmark_text}"
        );
        assert!(
            bookmark_text.contains("q-1"),
            "open_questions array preserved; got:\n{bookmark_text}"
        );

        dev.close().await;
    });
}

#[test]
fn get_bookmark_role_arg_overrides_default() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        pm.call_tool(
            "save_bookmark",
            serde_json::json!({
                "current_task": "PM_BOOKMARK_MARKER",
                "next_step": "pm-specific-next-step"
            }),
        )
        .await
        .expect("save_bookmark on pm should succeed");

        let pm_via_dev_override = dev
            .call_tool("get_bookmark", serde_json::json!({"role": "pm"}))
            .await
            .expect("get_bookmark with role=pm should succeed");

        let payload_text = pm_via_dev_override
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| pm_via_dev_override.to_string());
        assert!(
            payload_text.contains("PM_BOOKMARK_MARKER"),
            "role-override surfaces pm's bookmark from dev's bridge; got:\n{payload_text}"
        );

        pm.close().await;
        dev.close().await;
    });
}
