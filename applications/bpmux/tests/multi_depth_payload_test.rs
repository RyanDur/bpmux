//! End-to-end test for #187: a message with `multi_depth` sent by
//! one agent is received by another with the payload intact.
//!
//! Drives the real bpmux binary via the PTY harness, posts a
//! structured payload via the REST /api/messages endpoint, reads
//! it back via GET /api/messages, and asserts the parsed payload
//! round-trips. This is the behavior-layer guard that the schema
//! actually survives the bridge path end-to-end.
//!
//! Unit tests for `MultiDepthPayload` serde lived in
//! `src/session/message_tests.rs::multi_depth_*`. This test adds
//! the behavior layer — same extract-pure-core + drive-real-path
//! pattern as #179, #170, #181, #186.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

// (TestBpmux::start now waits for the HTTP listener bind itself —
// slice 14 + 19 — so per-test inline polls aren't needed; the
// earlier slice-7 helper was retired in slice 21.)

#[test]
fn multi_depth_payload_roundtrips_through_bridge_rest_path() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        // Post a message with a structured multi_depth payload — the
        // shape a future CI broadcast might use per #184's per-role
        // default-depth table.
        let payload = serde_json::json!({
            "from": "ci:bridge",
            "text": "CI \u{2717}",
            "multi_depth": {
                "summary": "CI \u{2717}",
                "role_default": {
                    "pm": "CI \u{2717} (run failed)",
                    "dev": "CI \u{2717} — run #12345 at cargo clippy, commit abc1234",
                    "designer": "CI \u{2717}"
                },
                "deep": {
                    "kind": "on_demand",
                    "command": "gh run view 12345",
                    "description": "open the full build log"
                }
            }
        });
        let resp = client
            .post(format!("http://127.0.0.1:{port}/api/messages"))
            .json(&payload)
            .send()
            .await
            .expect("POST /api/messages");
        assert!(
            resp.status().is_success(),
            "post should succeed, got {}",
            resp.status()
        );

        // Read it back and verify the payload survived serialization
        // → broadcast → storage → read.
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{port}/api/messages?chat_only=false"
        ))
        .await
        .expect("GET /api/messages")
        .text()
        .await
        .expect("read body");
        let messages: Vec<serde_json::Value> = serde_json::from_str(&resp).expect("parse messages");

        // Find the message we just posted.
        let found = messages
            .iter()
            .find(|m| m.get("multi_depth").and_then(|v| v.as_object()).is_some())
            .unwrap_or_else(|| panic!("multi_depth message missing. got:\n{resp}"));

        let md = found.get("multi_depth").expect("multi_depth");
        assert_eq!(md["summary"], "CI \u{2717}");
        assert_eq!(md["role_default"]["pm"], "CI \u{2717} (run failed)");
        assert_eq!(
            md["role_default"]["dev"],
            "CI \u{2717} — run #12345 at cargo clippy, commit abc1234"
        );
        assert_eq!(md["deep"]["kind"], "on_demand");
        assert_eq!(md["deep"]["command"], "gh run view 12345");
        assert_eq!(md["deep"]["description"], "open the full build log");
    });
}

#[test]
fn message_without_multi_depth_still_roundtrips_unchanged() {
    // Backward-compat guard: existing callers that don't send
    // multi_depth continue to work. The field is omitted from the
    // serialized form (serde skip_serializing_if).
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "from": "dev:claude-code",
            "text": "plain message with no multi_depth"
        });
        let resp = client
            .post(format!("http://127.0.0.1:{port}/api/messages"))
            .json(&payload)
            .send()
            .await
            .expect("POST /api/messages");
        assert!(resp.status().is_success());

        let resp = reqwest::get(format!(
            "http://127.0.0.1:{port}/api/messages?chat_only=false"
        ))
        .await
        .expect("GET /api/messages")
        .text()
        .await
        .expect("read body");
        let messages: Vec<serde_json::Value> = serde_json::from_str(&resp).expect("parse messages");
        let found = messages
            .iter()
            .find(|m| {
                m.get("text").and_then(|v| v.as_str()) == Some("plain message with no multi_depth")
            })
            .expect("message missing");
        // The multi_depth field should be absent from the serialized
        // form (not present as null or empty object).
        assert!(
            found.get("multi_depth").is_none(),
            "absent multi_depth should be omitted from serialized form, got: {found}"
        );
    });
}
