#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn relevance_set_filters_live_push_but_floor_and_escape_hatch_stay_open() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let sender = TestBridge::connect(&bpmux, "dev").await;
        let receiver = TestBridge::connect(&bpmux, "pm").await;

        receiver
            .call_tool("subscribe", serde_json::json!({ "event_class": "insight" }))
            .await
            .expect("an agent can declare a chat-relevance-set via subscribe");
        let observe = receiver.subscribe("broadcast_received").await;

        sender
            .call_tool(
                "send_message",
                serde_json::json!({ "text": "SUPPRESSED-CHAT-MARKER muted broadcast" }),
            )
            .await
            .expect("sender broadcasts a chat the receiver has muted");
        sender
            .call_tool(
                "send_message",
                serde_json::json!({
                    "text": "INSET-INSIGHT-BARRIER reasoning trace",
                    "message_type": "insight"
                }),
            )
            .await
            .expect("sender broadcasts an in-set insight");

        let pushed = receiver.wait_for_event(&observe, Duration::from_secs(5)).await;
        assert!(
            pushed.contains("INSET-INSIGHT-BARRIER"),
            "the in-set insight reaches the live broadcast_received push; got {pushed}"
        );
        assert!(
            !pushed.contains("SUPPRESSED-CHAT-MARKER"),
            "relevance-set {{insight}} mutes broadcast-chat from the live push — the muted chat was \
             sent FIRST, so if it pushed it would be the FIRST broadcast_received event; instead the \
             barrier insight is; got {pushed}"
        );

        sender
            .call_tool(
                "send_message",
                serde_json::json!({
                    "text": "FLOOR-ADDRESSED-CHAT direct to pm",
                    "addressee": "pm"
                }),
            )
            .await
            .expect("sender directs a chat at the receiver");
        let floor = receiver.wait_for_event(&observe, Duration::from_secs(5)).await;
        assert!(
            floor.contains("FLOOR-ADDRESSED-CHAT"),
            "an addressed chat is on the floor — it reaches the live push despite the mute; got {floor}"
        );

        let log = receiver
            .call_tool("read_messages", serde_json::json!({ "limit": 50 }))
            .await
            .expect("explicit pull stays open");
        assert!(
            log.to_string().contains("SUPPRESSED-CHAT-MARKER"),
            "the chat suppressed from the LIVE push is still retrievable via explicit read_messages \
             (escape-hatch preserved by construction — the filter lives only at the live-push); got {log}"
        );

        sender.close().await;
        receiver.close().await;
    });
}
