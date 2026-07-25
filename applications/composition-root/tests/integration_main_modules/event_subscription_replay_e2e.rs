#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, ensure_current_iteration};

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
fn missed_event_during_down_window_is_replayed_exactly_once_on_reconnect() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let mutator = TestBridge::connect(&bpmux, "pm").await;
        let agent = TestBridge::connect(&bpmux, "dev").await;

        ensure_current_iteration(&mutator).await;

        let target = create_target_story(
            &mutator,
            "[Feature] missed-event-replay-target",
            "reject if a down-window event is not replayed on reconnect",
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

        agent.close().await;

        mutator
            .call_tool(
                "move_story",
                serde_json::json!({
                    "id": target,
                    "target_state": "ready",
                    "rationale": "fire board_invalidated during the subscriber's down-window"
                }),
            )
            .await
            .expect("move_story (board mutation) succeeds while the subscriber is DOWN");

        let agent = TestBridge::connect(&bpmux, "dev").await;

        let replayed = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 5000 }),
            )
            .await
            .expect("the event missed during the down-window is replayed on reconnect");

        assert_eq!(
            replayed["event_class"].as_str(),
            Some("board_invalidated"),
            "an event fired while the agent's bridge was DOWN must be replayed on reconnect from the durable log; got {replayed}"
        );

        agent.close().await;
        let agent = TestBridge::connect(&bpmux, "dev").await;

        let after = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 1500 }),
            )
            .await
            .expect("second reconnect wait_for_event returns a signal");

        assert_eq!(
            after["timed_out"].as_bool(),
            Some(true),
            "exactly-once: the cursor must advance past the replayed event so a SECOND reconnect does NOT re-deliver it; got {after}"
        );

        agent.close().await;
        mutator.close().await;
    });
}
