//! Story 87 Acceptance Test: read_messages excludes activity noise by default.
//!
//! AC1: Default read_messages returns only chat messages
//! AC2: Full unfiltered log remains accessible

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN the session log contains chat and activity messages
/// WHEN read_messages is called (default: chat_only=true)
/// THEN only chat messages are returned
#[test]
fn default_read_excludes_activity() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;

        // Send a chat message
        dev.send("hello from dev").await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Send an activity message (status update)
        dev.send_activity("busy").await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Send another chat message
        dev.send("still working").await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Default API call — should exclude activity
        let resp = reqwest::get(format!("http://127.0.0.1:{port}/api/messages?limit=50"))
            .await
            .expect("GET /api/messages")
            .text()
            .await
            .expect("response body");

        let msgs: Vec<serde_json::Value> = serde_json::from_str(&resp).expect("parse JSON");

        // Should have chat messages but NOT activity
        let has_chat = msgs.iter().any(|m| m["text"] == "hello from dev");
        let has_activity = msgs
            .iter()
            .any(|m| m["message_type"].as_str() == Some("activity"));

        assert!(has_chat, "Should include chat messages. Got: {resp}");
        assert!(
            !has_activity,
            "Should exclude activity messages by default. Got: {resp}"
        );
    });
}

/// AC2: GIVEN the session log contains chat and activity messages
/// WHEN read_messages is called with chat_only=false
/// THEN all messages including activity are returned
#[test]
fn unfiltered_log_accessible() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;

        // Send chat + activity
        dev.send("chat msg").await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        dev.send_activity("busy").await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Explicit chat_only=false — should include everything
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{port}/api/messages?limit=50&chat_only=false"
        ))
        .await
        .expect("GET /api/messages?chat_only=false")
        .text()
        .await
        .expect("response body");

        let msgs: Vec<serde_json::Value> = serde_json::from_str(&resp).expect("parse JSON");

        let has_chat = msgs.iter().any(|m| m["text"] == "chat msg");
        let has_activity = msgs
            .iter()
            .any(|m| m["message_type"].as_str() == Some("activity"));

        assert!(has_chat, "Unfiltered should include chat. Got: {resp}");
        assert!(
            has_activity,
            "Unfiltered should include activity. Got: {resp}"
        );
    });
}
