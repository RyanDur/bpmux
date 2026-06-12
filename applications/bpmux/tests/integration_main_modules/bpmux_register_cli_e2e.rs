use std::process::Stdio;

use tempfile::TempDir;

use crate::harness::pty::TestBpmux;

async fn run_register(bin: &str, port: u16, cwd: &std::path::Path) -> (bool, String) {
    let output = tokio::process::Command::new(bin)
        .arg("register")
        .current_dir(cwd)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run bpmux register");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (output.status.success(), stdout)
}

#[tokio::test]
async fn register_cli_holds_the_id_in_a_dir_marker_and_makes_it_resolvable_server_side() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    let (ok, stdout) = run_register(bin, port, workspace.path()).await;
    assert!(
        ok,
        "bpmux register must succeed end-to-end against the live server (CLI -> /api/register -> store); stdout: {stdout}"
    );
    assert!(
        stdout.contains("registered dev"),
        "the CLI confirms the registration of this agent; stdout: {stdout}"
    );

    let marker = workspace.path().join(".bpmux-session-id");
    assert!(
        marker.is_file(),
        "register holds the agent's session-id in a lightweight .bpmux-session-id marker in the cwd (the dir-anchor), not a full session dir"
    );
    let session_id = std::fs::read_to_string(&marker)
        .expect("read marker")
        .trim()
        .to_string();
    assert!(
        !session_id.is_empty(),
        "the marker holds the minted session-id"
    );

    let store_json =
        std::fs::read_to_string(bpmux.sessions_base_path().join("routing_identity.json"))
            .expect("the server persists the routing-identity store after register");
    let store: serde_json::Value =
        serde_json::from_str(&store_json).expect("routing_identity.json is valid json");
    assert_eq!(
        store["display_by_id"][&session_id].as_str(),
        Some("dev"),
        "the registered id is RESOLVABLE server-side — the durable store maps the cwd-derived session-id to display 'dev' (a real routing effect, not just a 201); store: {store_json}"
    );
}

#[tokio::test]
async fn register_in_a_different_dir_mints_a_distinct_session_id() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace_a = TempDir::new().expect("temp workspace a");
    let workspace_b = TempDir::new().expect("temp workspace b");

    let (ok_a, _) = run_register(bin, port, workspace_a.path()).await;
    let (ok_b, _) = run_register(bin, port, workspace_b.path()).await;
    assert!(ok_a && ok_b, "both registrations succeed");

    let id_a = std::fs::read_to_string(workspace_a.path().join(".bpmux-session-id"))
        .expect("read marker a")
        .trim()
        .to_string();
    let id_b = std::fs::read_to_string(workspace_b.path().join(".bpmux-session-id"))
        .expect("read marker b")
        .trim()
        .to_string();
    assert_ne!(
        id_a, id_b,
        "a register from a different start-dir mints a DISTINCT session-id — the identity is anchored to the dir, so two agents started in different dirs are distinct instances"
    );
}
