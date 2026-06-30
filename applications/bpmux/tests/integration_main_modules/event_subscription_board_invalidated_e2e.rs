#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
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

#[test]
fn subscribe_then_non_create_board_mutation_delivers_board_invalidated() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;
        let mutator = TestBridge::connect(&bpmux, "pm").await;

        let target = create_target_story(
            &mutator,
            "[Feature] event-sub-move-target",
            "reject if the subscriber misses board_invalidated on a NON-create mutation",
        )
        .await;

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({ "event_class": "board_invalidated" }),
            )
            .await
            .expect("subscribe returns ok");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&agent, &subscription_id).await;

        mutator
            .call_tool(
                "move_story",
                serde_json::json!({
                    "id": target,
                    "target_state": "ready",
                    "rationale": "promote for the event-subscription coverage test"
                }),
            )
            .await
            .expect("move_story (a NON-create board mutation) succeeds");

        let event = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 5000 }),
            )
            .await
            .expect("wait_for_event returns the move's board_invalidated, not a timeout");

        assert_eq!(
            event["event_class"].as_str(),
            Some("board_invalidated"),
            "a NON-create (move) mutation must deliver board_invalidated — FULL coverage, not create-only (the silently-partial trap); got {event}"
        );
        assert!(
            event["payload"]["reason"].as_str().is_some(),
            "the delivered event carries a reason payload; got {event}"
        );

        agent.close().await;
        mutator.close().await;
    });
}

#[test]
fn wait_for_event_returns_a_timeout_signal_when_no_event_arrives() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({ "event_class": "board_invalidated" }),
            )
            .await
            .expect("subscribe returns ok");
        let subscription_id = sub["subscription_id"].as_str().unwrap().to_string();

        let event: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 300 }),
            )
            .await
            .expect("wait_for_event with no event must return a timeout signal, never block-without-recourse");

        assert_eq!(
            event["timed_out"].as_bool(),
            Some(true),
            "wait_for_event must return a bounded timeout signal (not hang) when no event arrives in the window; got {event}"
        );

        agent.close().await;
    });
}
