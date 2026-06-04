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
fn reconnect_replays_state_deltas_but_never_auto_replays_chat() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let mutator = TestBridge::connect(&bpmux, "pm").await;
        let agent = TestBridge::connect(&bpmux, "dev").await;

        let target = create_target_story(
            &mutator,
            "[Feature] chat-no-auto-replay-target",
            "reject if reconnect auto-replays chat — chat has no auto-replay surface",
        )
        .await;

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({ "event_class": "board_invalidated" }),
            )
            .await
            .expect("subscribe to the structured control event");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();
        agent
            .call_tool("subscribe", serde_json::json!({ "event_class": "chat" }))
            .await
            .expect("declare broadcast-chat in the relevance-set so it WOULD replay if a chat replay-surface existed");

        drain_pending_events(&agent, &subscription_id).await;

        agent.close().await;

        mutator
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target,
                    "target_state": "ready",
                    "rationale": "fire board_invalidated during the down-window (positive control)"
                }),
            )
            .await
            .expect("board mutation during the down-window");
        mutator
            .call_tool(
                "send_message",
                serde_json::json!({ "text": "MISSED-CHAT-MARKER during down-window" }),
            )
            .await
            .expect("broadcast a chat during the down-window");

        let agent = TestBridge::connect(&bpmux, "dev").await;

        let replayed = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 5000 }),
            )
            .await
            .expect("the down-window state-delta replays on reconnect");
        assert_eq!(
            replayed["event_class"].as_str(),
            Some("board_invalidated"),
            "positive control: the structured state-delta missed during the down-window MUST auto-replay on reconnect, proving the replay machinery is live and the test is non-vacuous; got {replayed}"
        );

        let pushed = agent.capture.received();
        assert!(
            !pushed.iter().any(|m| m.contains("MISSED-CHAT-MARKER")),
            "#1652 supersede-proof: chat has NO reconnect-replay surface — a chat broadcast during the down-window must NOT be auto-pushed on reconnect (the replay loop is state_delta-scoped). The state-delta already replayed above, so a chat-replay would already be present if the surface existed; got {pushed:?}"
        );

        let log = agent
            .call_tool("read_messages", serde_json::json!({ "limit": 50 }))
            .await
            .expect("explicit pull");
        assert!(
            log.to_string().contains("MISSED-CHAT-MARKER"),
            "escape-hatch: chat missed during the down-window is recovered via explicit read_messages, not auto-replay — the awareness-design catch-up path; got {log}"
        );

        agent.close().await;
        mutator.close().await;
    });
}
