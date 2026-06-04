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

        receiver
            .capture
            .wait_for("INSET-INSIGHT-BARRIER", Duration::from_secs(5))
            .await;

        let pushed = receiver.capture.received();
        assert!(
            !pushed.iter().any(|m| m.contains("SUPPRESSED-CHAT-MARKER")),
            "relevance-set {{insight}} mutes broadcast-chat from the live push — the barrier insight \
             arrived (FIFO after the chat), so a pushed chat would already be present; got {pushed:?}"
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
        receiver
            .capture
            .wait_for("FLOOR-ADDRESSED-CHAT", Duration::from_secs(5))
            .await;

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
