use std::process::Stdio;

use tempfile::TempDir;

use crate::harness::pty::TestBpmux;

async fn register(bin: &str, port: u16, data_base: &std::path::Path, cwd: &std::path::Path) {
    let output = tokio::process::Command::new(bin)
        .arg("register")
        .current_dir(cwd)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_DATA_BASE", data_base)
        .env("BPMUX_ROLE", "dev")
        .env_remove("BPMUX_USER_ID")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run bpmux register");
    assert!(
        output.status.success(),
        "register must succeed before wake-on-event can resolve a dir-held user-id; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn wake_on_event_cli_commits_a_filtered_gate_against_the_running_server() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let data_base = bpmux.sessions_base_path();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    register(bin, port, &data_base, workspace.path()).await;

    let output = tokio::process::Command::new(bin)
        .args([
            "wake-on-event",
            "--event",
            "ci_state_changed",
            "--filter",
            "{\"conclusion\":\"green\"}",
        ])
        .current_dir(workspace.path())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("BPMUX_DATA_BASE", &data_base)
        .env_remove("BPMUX_USER_ID")
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
    let data_base = bpmux.sessions_base_path();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    let output = tokio::process::Command::new(bin)
        .args([
            "wake-on-event",
            "--event",
            "ci_state_changed",
            "--filter",
            "\"green\"",
        ])
        .current_dir(workspace.path())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("BPMUX_DATA_BASE", &data_base)
        .env_remove("BPMUX_USER_ID")
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
        "the rejection names the contract (a JSON object) so the agent can correct it — this check fires BEFORE identity resolution, so an unregistered workspace still rejects on filter shape, not on the id-guard; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn wake_on_event_cli_errors_with_a_register_first_message_when_the_dir_has_no_registered_id()
{
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let data_base = bpmux.sessions_base_path();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    let output = tokio::process::Command::new(bin)
        .args(["wake-on-event", "--event", "ci_state_changed"])
        .current_dir(workspace.path())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("BPMUX_DATA_BASE", &data_base)
        .env_remove("BPMUX_USER_ID")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run bpmux wake-on-event");

    assert!(
        !output.status.success(),
        "a commitment without a resolvable id can never fire post-flip — wake-on-event must refuse, unlike send which silently omits sender_id"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("register"),
        "the rejection names the fix (register first) so the agent can correct it; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
