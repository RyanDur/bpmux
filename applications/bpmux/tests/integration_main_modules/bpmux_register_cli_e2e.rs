use std::process::Stdio;

use tempfile::TempDir;

use crate::harness::pty::TestBpmux;

async fn run_register(bin: &str, port: u16, cwd: &std::path::Path, role: &str) -> (bool, String) {
    let output = tokio::process::Command::new(bin)
        .arg("register")
        .current_dir(cwd)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", role)
        .env_remove("BPMUX_SESSION_ID")
        .env_remove("BPMUX_TERMINAL_ID")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run bpmux register");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (output.status.success(), stdout)
}

fn marker_id(marker: &std::path::Path) -> String {
    let raw = std::fs::read_to_string(marker).expect("read marker");
    serde_json::from_str::<serde_json::Value>(raw.trim())
        .ok()
        .and_then(|value| value["id"].as_str().map(str::to_string))
        .expect("the marker holds {id, name} JSON")
}

#[tokio::test]
async fn register_cli_holds_the_id_in_a_dir_marker_and_makes_it_resolvable_server_side() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    let (ok, stdout) = run_register(bin, port, workspace.path(), "dev").await;
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
        "register holds the agent's identity in a lightweight .bpmux-session-id marker in the cwd (the dir-anchor), not a full session dir"
    );
    let session_id = marker_id(&marker);
    assert!(
        !session_id.is_empty(),
        "the {{id, name}} marker holds the minted session-id"
    );

    use bpmux::session::log::StorageBackend;
    let session_dir = bpmux.sessions_base_path().join("current");
    let storage = StorageBackend::resolve().build(&session_dir);
    let users = storage
        .load_users()
        .expect("the server persists the routing-identity store after register");
    let user = users
        .iter()
        .find(|u| u.id == session_id)
        .unwrap_or_else(|| {
            panic!("the registered id is a durable row in the users table; got {users:?}")
        });
    assert_eq!(
        user.name, "dev",
        "the registered id is RESOLVABLE server-side — the durable users table maps the cwd-derived session-id to display 'dev' (a real routing effect persisted across restarts, not just a 201)"
    );
}

#[tokio::test]
async fn register_in_a_different_dir_mints_a_distinct_session_id() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace_a = TempDir::new().expect("temp workspace a");
    let workspace_b = TempDir::new().expect("temp workspace b");

    let (ok_a, _) = run_register(bin, port, workspace_a.path(), "dev").await;
    let (ok_b, _) = run_register(bin, port, workspace_b.path(), "pm").await;
    assert!(ok_a && ok_b, "both registrations succeed");

    let id_a = marker_id(&workspace_a.path().join(".bpmux-session-id"));
    let id_b = marker_id(&workspace_b.path().join(".bpmux-session-id"));
    assert_ne!(
        id_a, id_b,
        "a register from a different start-dir mints a DISTINCT session-id — the identity is anchored to the dir, so two agents started in different dirs are distinct instances"
    );
}

#[tokio::test]
async fn register_rejects_the_reserved_owner_identity() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    let (ok, stdout) = run_register(bin, port, workspace.path(), "owner").await;
    assert!(
        !ok,
        "registering the reserved OWNER identity must be REJECTED server-side (403) — an agent can never present as the owner; auto-submit's defense-in-depth peer-impersonation guard; stdout: {stdout}"
    );
}

#[tokio::test]
async fn register_with_a_terminal_reflects_the_routing_name_in_the_roster() {
    let bpmux = TestBpmux::start();
    let resp = bpmux
        .authed_post(
            "/api/register",
            serde_json::json!({
                "session_id": "sess-dev-roster",
                "display": "dev",
                "terminal_id": "abcd1234-5678-90ab-cdef-1234567890ab",
            }),
        )
        .await;
    assert!(resp.status().is_success(), "register should succeed");

    let url = format!("http://127.0.0.1:{}/api/roster", bpmux.port());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut identities: Vec<String> = vec![];
    while std::time::Instant::now() < deadline {
        let body: serde_json::Value = reqwest::Client::new()
            .get(&url)
            .bearer_auth(bpmux.simulate_secret())
            .header(reqwest::header::HOST, format!("127.0.0.1:{}", bpmux.port()))
            .send()
            .await
            .expect("GET /api/roster")
            .json()
            .await
            .expect("roster json");
        identities = body["roster"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e["identity"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if identities.iter().any(|i| i == "dev") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        identities.iter().any(|i| i == "dev"),
        "registering routing identity 'dev' with a terminal must surface 'dev' in the roster (the DIRECT/sidebar source), not only in chat — the acceptance criterion 'name reflects on every surface'; got {identities:?}"
    );
}
