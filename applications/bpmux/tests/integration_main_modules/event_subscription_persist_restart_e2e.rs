#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;

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
fn subscription_survives_bridge_process_restart_and_resumes_live_push() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let mutator = TestBridge::connect(&bpmux, "pm").await;
        let agent = TestBridge::connect(&bpmux, "dev").await;

        let target = create_target_story(
            &mutator,
            "[Feature] sub-persist-restart-target",
            "reject if a restarted agent's subscription does not resume live push",
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
        let agent = TestBridge::connect(&bpmux, "dev").await;

        agent
            .call_tool("set_name", serde_json::json!({ "name": "dev" }))
            .await
            .expect("restarted bridge reconnects and its WS handshake round-trips");

        mutator
            .call_tool(
                "move_story",
                serde_json::json!({
                    "id": target,
                    "target_state": "ready",
                    "rationale": "fire board_invalidated after the subscriber's bridge restarted"
                }),
            )
            .await
            .expect("move_story (a board mutation) succeeds");

        let event = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 5000 }),
            )
            .await
            .expect("the restored subscription must receive the post-restart event via resumed push");

        assert_eq!(
            event["event_class"].as_str(),
            Some("board_invalidated"),
            "a subscription must survive a bridge-process restart and resume live PUSH with no manual re-subscribe (received via the ORIGINAL subscription_id, not a poll); got {event}"
        );

        agent.close().await;
        mutator.close().await;
    });
}

#[test]
fn restored_subscription_preserves_its_filter_across_bridge_restart() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let sender = TestBridge::connect(&bpmux, "pm").await;
        let recipient = TestBridge::connect(&bpmux, "dev").await;

        let sub = recipient
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "addressed_to_me",
                    "filter": { "addressee": "dev" }
                }),
            )
            .await
            .expect("subscribe with a filter returns ok");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&recipient, &subscription_id).await;

        recipient.close().await;
        let recipient = TestBridge::connect(&bpmux, "dev").await;

        recipient
            .call_tool("set_name", serde_json::json!({ "name": "dev" }))
            .await
            .expect("restarted bridge reconnects and its WS handshake round-trips");

        sender
            .call_tool(
                "send_message",
                serde_json::json!({ "text": "designer, non-matching", "addressee": "designer" }),
            )
            .await
            .expect("send_message addressed to designer succeeds");
        sender
            .call_tool(
                "send_message",
                serde_json::json!({ "text": "dev, matching", "addressee": "dev" }),
            )
            .await
            .expect("send_message addressed to dev succeeds");

        let matched = recipient
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 20000 }),
            )
            .await
            .expect("the restored, filtered subscription receives the dev-addressed message via resumed push");

        assert_eq!(
            matched["event_class"].as_str(),
            Some("addressed_to_me"),
            "the restored subscription still delivers addressed_to_me after a bridge-process restart; got {matched}"
        );
        assert_eq!(
            matched["payload"]["addressee"].as_str(),
            Some("dev"),
            "the FILTER must survive the restart EXACTLY: the designer-addressed message was sent FIRST, so a lost or mangled filter would deliver it first and this would read 'designer'; got {matched}"
        );

        recipient.close().await;
        sender.close().await;
    });
}
