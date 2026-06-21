use std::process::Stdio;

use crate::harness::pty::TestBpmux;

#[tokio::test]
async fn wake_on_event_cli_commits_a_filtered_gate_against_the_running_server() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let output = tokio::process::Command::new(bin)
        .args([
            "wake-on-event",
            "--event",
            "ci_state_changed",
            "--filter",
            "{\"conclusion\":\"green\"}",
        ])
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run bpmux wake-on-event");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the real wake-on-event command commits against the running server (CLI parse → POST /api/wake-on-event → CommitmentStore); stdout: {stdout}, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("committed") && stdout.contains("ci_state_changed"),
        "the CLI confirms the committed gate (role + event + filter) off Channels; stdout: {stdout}"
    );
}

#[tokio::test]
async fn wake_on_event_cli_rejects_a_non_object_filter_without_touching_the_server() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let output = tokio::process::Command::new(bin)
        .args([
            "wake-on-event",
            "--event",
            "ci_state_changed",
            "--filter",
            "\"green\"",
        ])
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run bpmux wake-on-event");

    assert!(
        !output.status.success(),
        "a --filter that is valid JSON but not an OBJECT is rejected client-side — a bare string filter can never match the key/value payload semantics, so it is a usage error not a silent no-match"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("JSON object"),
        "the rejection names the contract (a JSON object) so the agent can correct it; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
