#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use bpmux::room::{BridgeWiring, terminal_provisioning};
use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, ensure_current_iteration};
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
fn provisioned_agent_receives_live_state_delta_push_not_just_rpc_tools() {
    let bpmux = TestBpmux::start();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let wiring = BridgeWiring {
        binary: bin.to_string(),
        port: bpmux.port(),
        agent_name: "agent-provtest".to_string(),
    };
    let integration = terminal_provisioning(
        "/bin/bash",
        Some(&wiring),
        &std::env::temp_dir().to_string_lossy(),
    )
    .expect("provisioning");
    let tempdir = integration
        .zdotdir
        .expect("bash provisioning yields a retained tempdir");
    let produced = std::fs::read_to_string(tempdir.path().join("bpmux-bridge.json"))
        .expect("the spawn must produce the bridge config the agent discovers");
    let cfg: Value = serde_json::from_str(&produced).expect("produced config is valid json");
    let server = &cfg["mcpServers"]["bpmux-bridge"];
    let program = server["command"].as_str().expect("command").to_string();
    let config_args: Vec<String> = server["args"]
        .as_array()
        .expect("produced config carries args")
        .iter()
        .map(|v| v.as_str().expect("string arg").to_string())
        .collect();

    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect_from_spawned_config(
            &bpmux,
            &program,
            &config_args,
            "agent-provtest",
        )
        .await;
        let mutator = TestBridge::connect(&bpmux, "pm").await;

        ensure_current_iteration(&mutator).await;

        let target = create_target_story(
            &mutator,
            "[Feature] provisioned-live-receive-target",
            "reject if a #1602-provisioned agent misses a live board_invalidated push",
        )
        .await;

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({ "event_class": "board_invalidated" }),
            )
            .await
            .expect("the provisioned agent can subscribe via its RPC tools");
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
                    "rationale": "fire board_invalidated for the provisioned subscriber"
                }),
            )
            .await
            .expect("move_story (board mutation) succeeds");

        let event = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 5000 }),
            )
            .await
            .expect("the provisioned agent receives the live board_invalidated via push");

        assert_eq!(
            event["event_class"].as_str(),
            Some("board_invalidated"),
            "a #1602-provisioned agent must receive live state-delta PUSH (via wait_for_event, not a poll), not just RPC tools; got {event}"
        );

        agent.close().await;
        mutator.close().await;
    });
}
