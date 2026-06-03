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

#[test]
fn subscribe_addressed_to_me_delivers_only_messages_addressed_to_my_role() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let recipient = TestBridge::connect(&bpmux, "dev").await;
        let sender = TestBridge::connect(&bpmux, "pm").await;

        let sub = recipient
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "addressed_to_me",
                    "filter": { "addressee": "dev" }
                }),
            )
            .await
            .expect("subscribe returns ok");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&recipient, &subscription_id).await;

        sender
            .call_tool(
                "send_message",
                serde_json::json!({
                    "text": "designer, please review the mock",
                    "addressee": "designer"
                }),
            )
            .await
            .expect("send_message (broadcast, addressed to designer) succeeds");

        let non_match: Value = recipient
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 2000 }),
            )
            .await
            .expect("wait_for_event returns");
        assert_eq!(
            non_match["timed_out"].as_bool(),
            Some(true),
            "a broadcast addressed to designer reaches dev's bridge but the addressee-filter rejects it — this is the discrimination proof; got {non_match}"
        );

        sender
            .call_tool(
                "send_message",
                serde_json::json!({
                    "text": "dev, can you pick up the next story",
                    "addressee": "dev"
                }),
            )
            .await
            .expect("send_message (broadcast, addressed to dev) succeeds");

        let matched = recipient
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 5000 }),
            )
            .await
            .expect("wait_for_event returns the dev-addressed message, not a timeout");

        assert_eq!(
            matched["event_class"].as_str(),
            Some("addressed_to_me"),
            "a message addressed to dev delivers an addressed_to_me event; got {matched}"
        );
        assert_eq!(
            matched["payload"]["addressee"].as_str(),
            Some("dev"),
            "the delivered event carries the addressee it was directed at; got {matched}"
        );
        assert_eq!(
            matched["payload"]["from"].as_str(),
            Some("pm"),
            "the delivered event carries the sender's short name; got {matched}"
        );
        assert!(
            matched["payload"]["text"]
                .as_str()
                .is_some_and(|t| !t.is_empty()),
            "the delivered event carries the message text; got {matched}"
        );
        assert!(
            matched["payload"]["ts"].as_str().is_some(),
            "the delivered event carries a timestamp; got {matched}"
        );
        assert!(
            matched["payload"]["channel"].as_str().is_some(),
            "the delivered event carries the channel it arrived on; got {matched}"
        );

        recipient.close().await;
        sender.close().await;
    });
}
