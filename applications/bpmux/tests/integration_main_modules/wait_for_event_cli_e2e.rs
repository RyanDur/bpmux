use serde_json::json;

use crate::harness::pty::TestBpmux;

#[tokio::test]
async fn wait_for_event_cli_returns_a_broadcast_off_channels() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

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

    let bin = env!("CARGO_BIN_EXE_bpmux");
    let output = tokio::process::Command::new(bin)
        .args(["wait-for-event", "--timeout-ms", "5000"])
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .output()
        .await
        .expect("run bpmux wait-for-event");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("broadcast_received"),
        "CLI should return a broadcast_received event off Channels; stdout: {stdout}"
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
}
