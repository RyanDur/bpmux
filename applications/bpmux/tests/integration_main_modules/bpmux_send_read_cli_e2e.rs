use serde_json::Value;

use crate::harness::pty::TestBpmux;

#[tokio::test]
async fn bpmux_send_reaches_the_channel_and_is_read_back_via_bpmux_read() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let sent = tokio::process::Command::new(bin)
        .args(["send", "standup at 10"])
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .output()
        .await
        .expect("run bpmux send");
    let send_stdout = String::from_utf8_lossy(&sent.stdout);
    assert!(
        send_stdout.contains("✓ sent"),
        "bpmux send should render a one-line confirm in the stream; stdout: {send_stdout}"
    );

    let read = tokio::process::Command::new(bin)
        .args(["read"])
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "pm")
        .output()
        .await
        .expect("run bpmux read");
    let read_stdout = String::from_utf8_lossy(&read.stdout);
    assert!(
        read_stdout.contains("dev: standup at 10"),
        "the send must reach the channel + be readable via the in-slice bpmux read round-trip (decoupled from the #1656 drain), attributed to the sender; stdout: {read_stdout}"
    );
}

#[tokio::test]
async fn bpmux_help_json_exposes_the_messaging_verbs_for_a_no_a_priori_agent() {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let out = tokio::process::Command::new(bin)
        .args(["help", "--json"])
        .output()
        .await
        .expect("run bpmux help --json");
    let index: Value =
        serde_json::from_slice(&out.stdout).expect("bpmux help --json must emit parseable JSON");
    let verbs: Vec<&str> = index["verbs"]
        .as_array()
        .expect("verbs array")
        .iter()
        .filter_map(|entry| entry["verb"].as_str())
        .collect();
    assert!(
        verbs.contains(&"send") && verbs.contains(&"read"),
        "discoverability: a no-a-priori agent must find send + read via the bpmux help --json verb-index (MCP advertises tools for free; the CLI re-creates discovery); verbs: {verbs:?}"
    );
}
