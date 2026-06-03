#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use crate::harness;

use bpmux::session::message::DeliveryStatus;
use harness::bridge::TestBridge;
use harness::peer::WsPeer;
use harness::pty::TestBpmux;

#[test]
fn addressed_message_to_agent_reports_delivered_then_seen_on_next_action() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;
        let mut sender = WsPeer::connect(&bpmux, "pm").await;

        sender.send_to("please pick up #1604", &["dev"]).await;

        let delivered = sender
            .recv_message_status(Duration::from_secs(5))
            .await
            .expect("a message addressed to a connected agent reports DELIVERED once its bridge forwards it");
        assert_eq!(
            delivered.status,
            DeliveryStatus::Delivered,
            "the first receipt is DELIVERED (reached the agent's transport), not SEEN; got {delivered:?}"
        );
        assert_eq!(
            delivered.recipient, "dev",
            "the receipt names the recipient agent; got {delivered:?}"
        );
        let message_id = delivered.message_id.clone();
        assert!(
            !message_id.is_empty(),
            "the receipt carries the server-assigned message id used to correlate seen; got {delivered:?}"
        );

        let premature = sender.recv_message_status(Duration::from_millis(500)).await;
        assert!(
            premature
                .as_ref()
                .is_none_or(|s| s.status != DeliveryStatus::Seen),
            "SEEN must NOT fire before the agent acts on its turn — delivered-not-seen is the in-flight \
             state this feature makes legible; got a premature seen: {premature:?}"
        );

        agent
            .call_tool("get_identity", serde_json::json!({}))
            .await
            .expect("the agent takes its next action at the turn-boundary");

        let seen = sender
            .recv_message_status(Duration::from_secs(5))
            .await
            .expect("the agent's next action reports SEEN — never a silent drop");
        assert_eq!(
            seen.status,
            DeliveryStatus::Seen,
            "acting at the turn-boundary transitions the receipt to SEEN; got {seen:?}"
        );
        assert_eq!(
            seen.message_id, message_id,
            "SEEN references the same message that was DELIVERED, not a different one; got {seen:?}"
        );

        agent.close().await;
        sender.close().await;
    });
}

#[test]
fn broadcast_message_emits_no_delivered_seen_receipts() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;
        let mut observer = WsPeer::connect(&bpmux, "pm").await;

        observer.send("standup in five").await;

        agent
            .call_tool("get_identity", serde_json::json!({}))
            .await
            .expect("agent acts");

        let receipt = observer
            .recv_message_status(Duration::from_millis(750))
            .await;
        assert!(
            receipt.is_none(),
            "delivered/seen is scoped to messages ADDRESSED to a specific agent — a broadcast \
             must emit no per-recipient receipts; got {receipt:?}"
        );

        agent.close().await;
        observer.close().await;
    });
}
