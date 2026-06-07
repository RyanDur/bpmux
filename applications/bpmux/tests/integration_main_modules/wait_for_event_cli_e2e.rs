use std::process::Stdio;
use std::time::Duration;

use serde_json::json;

use crate::harness::pty::TestBpmux;

#[tokio::test]
async fn wait_for_event_cli_returns_a_broadcast_that_arrives_while_it_blocks() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let waiter = tokio::process::Command::new(bin)
        .args(["wait-for-event", "--timeout-ms", "5000"])
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bpmux wait-for-event");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let posted = bpmux
        .authed_post(
            "/api/messages",
            json!({ "from": "pm", "text": "hello dev via cli" }),
        )
        .await;
    assert!(
        posted.status().is_success(),
        "broadcast POST should succeed, got {}",
        posted.status()
    );

    let output = waiter
        .wait_with_output()
        .await
        .expect("collect bpmux wait-for-event output");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("broadcast_received"),
        "the blocking CLI should return the broadcast that arrived WHILE it was waiting, off Channels; stdout: {stdout}"
    );
    assert!(
        stdout.contains("hello dev via cli"),
        "CLI should carry the broadcast text; stdout: {stdout}"
    );
    assert!(
        stdout.contains("\"from\":\"pm\""),
        "CLI should attribute the sender; stdout: {stdout}"
    );
    assert!(
        stdout.contains("\"cursor\""),
        "CLI should return a cursor for the next pull; stdout: {stdout}"
    );
    assert!(
        !stdout.contains("timed_out"),
        "CLI should not time out — the event arrived inside the wait window; stdout: {stdout}"
    );
}
