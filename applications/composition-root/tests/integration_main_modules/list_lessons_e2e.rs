#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn list_lessons_empty_session_returns_empty_or_no_results() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("list_lessons", serde_json::json!({}))
            .await
            .expect("list_lessons should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("[]")
                || text.contains("\"lessons\":[]")
                || text.is_empty()
                || text.contains("0"),
            "empty session returns empty/zero result; got `{text}`"
        );

        dev.close().await;
    });
}

#[test]
fn list_lessons_topic_filter_returns_only_matching_lessons() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        dev.call_tool(
            "save_lesson",
            serde_json::json!({
                "topic": "topic-A-list-lessons-filter",
                "lesson": "LESSON_A_BODY"
            }),
        )
        .await
        .expect("save_lesson A should succeed");

        dev.call_tool(
            "save_lesson",
            serde_json::json!({
                "topic": "topic-B-list-lessons-filter",
                "lesson": "LESSON_B_BODY"
            }),
        )
        .await
        .expect("save_lesson B should succeed");

        let filtered = dev
            .call_tool(
                "list_lessons",
                serde_json::json!({"topic": "topic-A-list-lessons-filter"}),
            )
            .await
            .expect("list_lessons with topic filter should succeed");

        let filtered_text = filtered
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| filtered.to_string());
        assert!(
            filtered_text.contains("LESSON_A_BODY"),
            "topic filter returns matching lesson; got:\n{filtered_text}"
        );
        assert!(
            !filtered_text.contains("LESSON_B_BODY"),
            "topic filter excludes non-matching lessons; got:\n{filtered_text}"
        );

        dev.close().await;
    });
}

#[test]
fn list_lessons_returns_all_lessons_without_filter() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let marker_x = "LIST_LESSONS_NO_FILTER_X1";
        let marker_y = "LIST_LESSONS_NO_FILTER_Y1";

        dev.call_tool(
            "save_lesson",
            serde_json::json!({
                "topic": "no-filter-topic-x",
                "lesson": marker_x
            }),
        )
        .await
        .expect("save_lesson X should succeed");

        dev.call_tool(
            "save_lesson",
            serde_json::json!({
                "topic": "no-filter-topic-y",
                "lesson": marker_y
            }),
        )
        .await
        .expect("save_lesson Y should succeed");

        let all = dev
            .call_tool("list_lessons", serde_json::json!({}))
            .await
            .expect("list_lessons should succeed");

        let all_text = all
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| all.to_string());
        assert!(
            all_text.contains(marker_x),
            "list_lessons without filter includes lesson X; got:\n{all_text}"
        );
        assert!(
            all_text.contains(marker_y),
            "list_lessons without filter includes lesson Y; got:\n{all_text}"
        );

        dev.close().await;
    });
}
