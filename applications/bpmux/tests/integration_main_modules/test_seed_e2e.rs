//! E2E test for the #1290 Part A seed endpoints. Verifies:
//! - Endpoints mount only when `BPMUX_TEST_SEED_API=1`.
//! - Caller-supplied `ts` round-trips through the active Storage trait.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness::pty::TestBpmux;
use chrono::{DateTime, Utc};
use serde_json::json;

async fn fetch_messages(port: u16) -> Vec<serde_json::Value> {
    let url = format!("http://127.0.0.1:{port}/api/messages?chat_only=false");
    let body = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .expect("GET /api/messages")
        .text()
        .await
        .expect("body");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    parsed.as_array().cloned().unwrap_or_default()
}

#[test]
fn seed_messages_preserves_caller_ts_through_sqlite_backend() {
    let bpmux = TestBpmux::start_with_envs(
        "human",
        &[
            ("BPMUX_TEST_SEED_API", "1"),
            ("BPMUX_STORAGE_BACKEND", "sqlite"),
        ],
    );
    let port = bpmux.port();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let ts_1: DateTime<Utc> = "2026-05-11T10:00:00Z".parse().unwrap();
        let ts_2: DateTime<Utc> = "2026-05-11T11:00:00Z".parse().unwrap();
        let payload = json!({
            "messages": [
                {"from": "dev", "text": "first", "ts": ts_1},
                {"from": "pm", "text": "second", "ts": ts_2}
            ]
        });
        let url = format!("http://127.0.0.1:{port}/api/test/seed_messages");
        let resp = reqwest::Client::new()
            .post(&url)
            .json(&payload)
            .send()
            .await
            .expect("seed_messages POST");
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        assert!(status.is_success(), "seed status: {status} body: {body}");

        let messages = fetch_messages(port).await;
        assert_eq!(
            messages.len(),
            2,
            "both seeds observable; seed body: {body}; got: {messages:?}"
        );
        // /api/messages returns newest-first; index 0 is ts_2.
        // Parse ts back to DateTime<Utc> for semantic equality
        // (different RFC3339 formatters use `Z` vs `+00:00` for the
        // same instant; ts-precision means SAME INSTANT round-trips,
        // not byte-identical string).
        let got_ts_0: DateTime<Utc> = messages[0]["ts"].as_str().unwrap().parse().unwrap();
        let got_ts_1: DateTime<Utc> = messages[1]["ts"].as_str().unwrap().parse().unwrap();
        assert_eq!(messages[0]["text"], "second");
        assert_eq!(got_ts_0, ts_2);
        assert_eq!(messages[1]["text"], "first");
        assert_eq!(got_ts_1, ts_1);
    });
}

#[test]
fn seed_messages_endpoint_absent_without_env_var() {
    // BPMUX_TEST_SEED_API NOT set — route should 404.
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let url = format!("http://127.0.0.1:{port}/api/test/seed_messages");
        let resp = reqwest::Client::new()
            .post(&url)
            .json(&json!({"messages": []}))
            .send()
            .await
            .expect("POST");
        assert_eq!(
            resp.status().as_u16(),
            404,
            "endpoint must not exist outside test-seed mode"
        );
    });
}
