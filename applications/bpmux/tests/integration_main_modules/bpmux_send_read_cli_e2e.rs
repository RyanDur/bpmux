use serde_json::Value;
use tempfile::TempDir;

use crate::harness::pty::TestBpmux;

#[tokio::test]
async fn bpmux_send_reaches_the_channel_and_is_read_back_via_bpmux_read() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    let sent = tokio::process::Command::new(bin)
        .args(["send", "standup at 10"])
        .current_dir(workspace.path())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("BPMUX_DATA_BASE", bpmux.data_base_path())
        .env_remove("BPMUX_USER_ID")
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
        .current_dir(workspace.path())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "pm")
        .env("BPMUX_DATA_BASE", bpmux.data_base_path())
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
async fn bpmux_send_identity_io_stays_inside_the_injected_data_base_never_the_default_home() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");
    let fake_home = TempDir::new().expect("fake home");
    std::fs::write(
        workspace.path().join(".bpmux-user-id"),
        r#"{"id":"import-me-e2e","name":"marked-workspace"}"#,
    )
    .expect("seed a pollution-era marker in the send cwd");

    let sent = tokio::process::Command::new(bin)
        .args(["send", "hello from a marked dir"])
        .current_dir(workspace.path())
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .env("BPMUX_DATA_BASE", bpmux.data_base_path())
        .env("HOME", fake_home.path())
        .env_remove("BPMUX_USER_ID")
        .output()
        .await
        .expect("run bpmux send");
    assert!(
        sent.status.success(),
        "send from a marked dir succeeds: {}",
        String::from_utf8_lossy(&sent.stderr)
    );

    assert!(
        !fake_home.path().join(".bpmux").exists(),
        "the anti-pollution invariant: a test-spawned send must NEVER fall through to the DEFAULT data home (~/.bpmux) — every identity read/import lands in the injected BPMUX_DATA_BASE (this is the leak class that wrote a real ~/.bpmux/data/users/by-dir entry during a test run)"
    );
    let sandbox_registry = bpmux.data_base_path().join("users").join("by-dir");
    let entries: Vec<_> = std::fs::read_dir(&sandbox_registry)
        .expect("the marker import landed in the INJECTED base's registry")
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "exactly the one imported entry lives in the sandboxed registry"
    );
    assert!(
        !workspace.path().join(".bpmux-user-id").exists(),
        "the marker was imported-then-deleted (cleanup canon) — into the sandbox, not the real home"
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
