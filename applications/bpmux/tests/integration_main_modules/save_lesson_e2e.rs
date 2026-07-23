#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn save_lesson_returns_topic_in_message() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool(
                "save_lesson",
                serde_json::json!({
                    "topic": "e2e-save-lesson-marker-topic",
                    "lesson": "When testing memory-archetype E2E, dev needs the lesson to roundtrip via real subprocess + /api/lessons."
                }),
            )
            .await
            .expect("save_lesson should succeed");

        let msg = result
            .as_str()
            .or_else(|| result["message"].as_str())
            .expect("save_lesson returns a message");
        assert!(
            msg.contains("lesson saved:"),
            "happy-path message names persistence; got {msg}"
        );
        assert!(
            msg.contains("e2e-save-lesson-marker-topic"),
            "message includes the saved topic verbatim; got {msg}"
        );

        dev.close().await;
    });
}

#[test]
fn save_lesson_rejects_missing_topic() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool(
                "save_lesson",
                serde_json::json!({
                    "lesson": "lesson body without a topic field"
                }),
            )
            .await
            .expect_err("missing topic should be rejected");

        assert!(
            err.contains("topic"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn save_lesson_rejects_missing_lesson() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool(
                "save_lesson",
                serde_json::json!({
                    "topic": "topic-without-lesson-field"
                }),
            )
            .await
            .expect_err("missing lesson should be rejected");

        assert!(
            err.contains("lesson"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn save_lesson_list_lessons_reflects_new_lesson_post_save() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let lesson_marker = "ROUNDTRIP_LESSON_MARKER_L1";
        dev.call_tool(
            "save_lesson",
            serde_json::json!({
                "topic": "e2e-save-lesson-roundtrip",
                "lesson": lesson_marker
            }),
        )
        .await
        .expect("save_lesson should succeed");

        let listed = dev
            .call_tool("list_lessons", serde_json::json!({}))
            .await
            .expect("list_lessons should succeed");
        let listed_text = listed
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| listed.to_string());
        assert!(
            listed_text.contains(lesson_marker),
            "list_lessons reflects the newly-saved lesson via real MCP roundtrip; got:\n{listed_text}"
        );

        dev.close().await;
    });
}
