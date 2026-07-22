use std::process::Stdio;

use tempfile::TempDir;

use crate::harness::pty::TestBpmux;

async fn run_register(
    bin: &str,
    port: u16,
    data_base: &std::path::Path,
    cwd: &std::path::Path,
    role: &str,
) -> (bool, String) {
    let output = tokio::process::Command::new(bin)
        .arg("register")
        .current_dir(cwd)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_DATA_BASE", data_base)
        .env("BPMUX_ROLE", role)
        .env_remove("BPMUX_USER_ID")
        .env_remove("BPMUX_TERMINAL_ID")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run bpmux register");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (output.status.success(), stdout)
}

fn registered_id(stdout: &str) -> String {
    let start = stdout
        .find('(')
        .expect("register stdout names the id in parens");
    let end = stdout[start..]
        .find(')')
        .expect("register stdout closes the id parens")
        + start;
    stdout[start + 1..end].to_string()
}

#[tokio::test]
async fn register_cli_holds_a_stable_id_in_the_dirs_own_marker_and_makes_it_resolvable_server_side()
{
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let data_base = bpmux.data_base_path();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    let (ok, stdout) = run_register(bin, port, &data_base, workspace.path(), "dev").await;
    assert!(
        ok,
        "bpmux register must succeed end-to-end against the live server (CLI -> /api/register -> store); stdout: {stdout}"
    );
    assert!(
        stdout.contains("registered dev"),
        "the CLI confirms the registration of this agent; stdout: {stdout}"
    );
    let user_id = registered_id(&stdout);
    assert!(!user_id.is_empty(), "register mints a user-id for the dir");

    assert!(
        workspace.path().join(".bpmux-user-id").exists(),
        "register mints the uuid into the start-dir's own marker — the dir IS the agent, so identity travels with the directory instead of a machine-wide registry"
    );
    assert!(
        !data_base.join("users").join("by-dir").exists(),
        "no central registry entry is created"
    );

    let (ok_again, stdout_again) =
        run_register(bin, port, &data_base, workspace.path(), "dev").await;
    assert!(ok_again, "a second register in the same dir succeeds");
    assert_eq!(
        registered_id(&stdout_again),
        user_id,
        "registering twice in the same dir RECLAIMS the same id — id stability is the observable behavior, not any storage filename"
    );

    use storage::StorageBackend;
    let storage = StorageBackend::resolve().build(&bpmux.data_base_path());
    let users = storage
        .load_users()
        .expect("the server persists the routing-identity store after register");
    let user = users
        .iter()
        .find(|u| u.user_id == user_id)
        .unwrap_or_else(|| {
            panic!("the registered id is a durable row in the users table; got {users:?}")
        });
    assert_eq!(
        user.name, "dev",
        "the registered id is RESOLVABLE server-side — the durable users table maps the cwd-derived user-id to display 'dev' (a real routing effect persisted across restarts, not just a 201)"
    );
}

#[tokio::test]
async fn register_in_a_different_dir_mints_a_distinct_user_id() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let data_base = bpmux.data_base_path();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace_a = TempDir::new().expect("temp workspace a");
    let workspace_b = TempDir::new().expect("temp workspace b");

    let (ok_a, stdout_a) = run_register(bin, port, &data_base, workspace_a.path(), "dev").await;
    let (ok_b, stdout_b) = run_register(bin, port, &data_base, workspace_b.path(), "pm").await;
    assert!(ok_a && ok_b, "both registrations succeed");

    let id_a = registered_id(&stdout_a);
    let id_b = registered_id(&stdout_b);
    assert_ne!(
        id_a, id_b,
        "a register from a different start-dir mints a DISTINCT user-id — the identity is anchored to the dir, so two agents started in different dirs are distinct instances"
    );
}

#[tokio::test]
async fn register_rejects_the_reserved_owner_identity() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let data_base = bpmux.data_base_path();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let workspace = TempDir::new().expect("temp workspace");

    let (ok, stdout) = run_register(bin, port, &data_base, workspace.path(), "owner").await;
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
                "user_id": "sess-dev-roster",
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
