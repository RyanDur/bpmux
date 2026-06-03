#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use serde_json::Value;

async fn drain_pending_events(agent: &TestBridge, subscription_id: &str) {
    loop {
        let drained = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 500 }),
            )
            .await
            .expect("drain wait_for_event");
        if drained["timed_out"].as_bool() == Some(true) {
            break;
        }
    }
}

async fn fire_sync_status(bpmux: &TestBpmux, status: &str, count: u64) {
    let response = bpmux
        .authed_post(
            "/api/test/fire_sync_status",
            serde_json::json!({
                "status": status,
                "count": count,
            }),
        )
        .await;
    assert!(
        response.status().is_success(),
        "fire_sync_status test-route should accept the transition; got {}",
        response.status()
    );
}

#[test]
fn subscribe_filtered_receives_only_matching_sync_transition() {
    let bpmux = TestBpmux::start_with_envs("human", &[("BPMUX_TEST_SEED_API", "1")]);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "sync_status_changed",
                    "filter": { "status": "drain" }
                }),
            )
            .await
            .expect("subscribe returns ok");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&agent, &subscription_id).await;

        fire_sync_status(&bpmux, "conflict", 2).await;
        fire_sync_status(&bpmux, "drain", 3).await;

        let matched: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 20000 }),
            )
            .await
            .expect("wait_for_event returns the matching sync transition, not a timeout");

        assert_eq!(
            matched["event_class"].as_str(),
            Some("sync_status_changed"),
            "the matching transition delivers a sync_status_changed event; got {matched}"
        );
        assert_eq!(
            matched["payload"]["status"].as_str(),
            Some("drain"),
            "the FIRST received event must be the drain transition — discrimination proof under any load: the conflict transition was fired FIRST, so were the status-filter to leak it the order-preserved dispatch would deliver conflict FIRST and this assert would read 'conflict' (no negative-timeout to outrun under CPU-starvation); got {matched}"
        );
        assert_eq!(
            matched["payload"]["pending_events"].as_u64(),
            Some(3),
            "the delivered event carries the transition's pending-event count; got {matched}"
        );
        assert!(
            matched["payload"]["why"]
                .as_str()
                .is_some_and(|why| !why.is_empty()),
            "the delivered event carries a non-empty 'why' explaining the transition; got {matched}"
        );

        agent.close().await;
    });
}
