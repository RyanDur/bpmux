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

async fn fire_ci_state(bpmux: &TestBpmux, run_id: u64, head_sha: &str, conclusion: &str) {
    let response = bpmux
        .authed_post(
            "/api/test/fire_ci_state",
            serde_json::json!({
                "run_id": run_id,
                "head_sha": head_sha,
                "conclusion": conclusion,
            }),
        )
        .await;
    assert!(
        response.status().is_success(),
        "fire_ci_state test-route should accept the transition; got {}",
        response.status()
    );
}

#[test]
fn subscribe_filtered_receives_only_matching_ci_transition() {
    let bpmux = TestBpmux::start_with_envs("human", &[("BPMUX_TEST_SEED_API", "1")]);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;

        let sha_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let sha_b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "ci_state_changed",
                    "filter": { "head_sha": sha_a }
                }),
            )
            .await
            .expect("subscribe returns ok");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&agent, &subscription_id).await;

        fire_ci_state(&bpmux, 2, sha_b, "failure").await;
        fire_ci_state(&bpmux, 1, sha_a, "success").await;

        let matched: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 20000 }),
            )
            .await
            .expect("wait_for_event returns the matching ci transition, not a timeout");

        assert_eq!(
            matched["event_class"].as_str(),
            Some("ci_state_changed"),
            "the matching head_sha transition delivers a ci_state_changed event; got {matched}"
        );
        assert_eq!(
            matched["payload"]["head_sha"].as_str(),
            Some(sha_a),
            "the FIRST received event must be the sha_a transition — discrimination proof under any load: the sha_b transition was fired FIRST, so were the head_sha-filter to leak it the order-preserved dispatch would deliver sha_b FIRST and this assert would read sha_b (no negative-timeout to outrun under CPU-starvation); got {matched}"
        );
        assert_eq!(
            matched["payload"]["run_id"].as_u64(),
            Some(1),
            "the delivered event carries the transition's run_id; got {matched}"
        );
        assert_eq!(
            matched["payload"]["conclusion"].as_str(),
            Some("green"),
            "the delivered event carries the serialized CiState conclusion; got {matched}"
        );
        assert!(
            matched["payload"]["url"].as_str().is_some(),
            "the delivered event carries a where-to-look url; got {matched}"
        );

        agent.close().await;
    });
}
