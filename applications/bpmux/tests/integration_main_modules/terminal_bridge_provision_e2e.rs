#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use bpmux::room::{BridgeWiring, terminal_provisioning};
use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use serde_json::Value;
use std::time::Duration;

#[test]
fn provisioned_bridge_config_is_session_pointed_and_the_spawned_config_connects_and_sends() {
    let bpmux = TestBpmux::start();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let wiring = BridgeWiring {
        binary: bin.to_string(),
        port: bpmux.port(),
        agent_name: "agent-seamtest".to_string(),
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
        .expect("the spawn must PRODUCE the bridge config the agent discovers");
    let cfg: Value = serde_json::from_str(&produced).expect("produced config is valid json");
    let server = &cfg["mcpServers"]["bpmux-bridge"];

    assert_eq!(
        server["command"].as_str(),
        Some(bin),
        "produced config must self-anchor at THIS bpmux binary so the agent reconnects to the LIVE session, not a stale path"
    );
    let config_args: Vec<String> = server["args"]
        .as_array()
        .expect("produced config carries args")
        .iter()
        .map(|v| v.as_str().expect("string arg").to_string())
        .collect();
    let port_s = bpmux.port().to_string();
    assert_eq!(
        config_args,
        vec![
            "bridge".to_string(),
            "--port".to_string(),
            port_s,
            "--name".to_string(),
            "agent-seamtest".to_string(),
        ],
        "produced config must carry the LIVE session port + a legible per-terminal --name (collision-free across N terminals)"
    );

    let program = server["command"].as_str().expect("command").to_string();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect_from_spawned_config(
            &bpmux,
            &program,
            &config_args,
            "agent-seamtest",
        )
        .await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        agent
            .call_tool(
                "send_message",
                serde_json::json!({ "text": "SEAM_PROVISION_CONNECT_MARKER_1602" }),
            )
            .await
            .expect(
                "the EXACT spawn-produced config, when run, must connect to the session and send",
            );

        agent.close().await;
    });
}

#[test]
fn terminal_spawn_endpoint_with_the_port_extension_returns_a_terminal_id_not_500() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let resp = bpmux
            .authed_post("/api/terminals/spawn", serde_json::json!({}))
            .await;
        assert_eq!(
            resp.status(),
            200,
            "terminal-spawn must not 500 — a missing or misw-ired port Extension (the shared-router change feeding the bridge config) surfaces here"
        );
        let body: Value = resp.json().await.expect("spawn returns json");
        assert!(
            body["terminal_id"].as_str().is_some_and(|s| !s.is_empty()),
            "spawn must return a non-empty terminal_id; got {body}"
        );
    });
}
