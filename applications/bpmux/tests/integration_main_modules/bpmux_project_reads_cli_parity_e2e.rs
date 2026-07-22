#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use tempfile::TempDir;

use crate::harness;

use harness::pty::TestBpmux;

async fn run_verb(
    bpmux: &TestBpmux,
    workspace: &std::path::Path,
    args: &[&str],
    env: &[(&str, &str)],
) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let mut cmd = tokio::process::Command::new(bin);
    cmd.args(args);
    cmd.current_dir(workspace);
    cmd.env("BPMUX_DATA_BASE", bpmux.data_base_path());
    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd.output()
        .await
        .expect("run bpmux project-read verb subprocess")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_sync_status_reaches_the_authed_rest_read_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let workspace = TempDir::new().expect("temp workspace");
    let token_path = bpmux.auth_token_path();
    let out = run_verb(
        &bpmux,
        workspace.path(),
        &["sync-status"],
        &[
            ("BPMUX_PORT", &bpmux.port().to_string()),
            ("BPMUX_ROLE", "dev"),
            (
                "BPMUX_AUTH_TOKEN_PATH",
                token_path.to_str().expect("token path"),
            ),
        ],
    )
    .await;
    assert!(
        out.status.success(),
        "bpmux sync-status reaches the same authed /api/sync-status read the bridge uses (a 401 would fail); stderr: {}",
        stderr(&out)
    );
    let parsed: serde_json::Value = serde_json::from_str(stdout(&out).trim())
        .expect("sync-status returns parseable JSON off MCP");
    assert!(
        parsed.is_object(),
        "sync-status returns the SyncStatus contract off MCP (non-vacuous: a status object, not empty / a route-404); stdout: {}",
        stdout(&out)
    );
}

#[tokio::test]
async fn bpmux_search_finds_a_sent_message_off_mcp_end_to_end() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let workspace = TempDir::new().expect("temp workspace");
    let port = bpmux.port().to_string();
    let token = bpmux.auth_token_path();
    let token_str = token.to_str().expect("token path").to_string();
    let env: Vec<(&str, &str)> = vec![
        ("BPMUX_PORT", port.as_str()),
        ("BPMUX_ROLE", "dev"),
        ("BPMUX_AUTH_TOKEN_PATH", token_str.as_str()),
    ];
    let needle = "zzqueryneedle";

    let sent = run_verb(&bpmux, workspace.path(), &["send", needle], &env).await;
    assert!(
        sent.status.success(),
        "bpmux send posts the message off MCP; stderr: {}",
        stderr(&sent)
    );

    let out = run_verb(&bpmux, workspace.path(), &["search", needle], &env).await;
    assert!(
        out.status.success(),
        "bpmux search reaches the same /api/search read the bridge uses off MCP; stderr: {}",
        stderr(&out)
    );
    assert!(
        stdout(&out).contains(needle),
        "search surfaces the message that bpmux send posted — a full off-MCP round trip (non-vacuous: the needle is in the results, not an empty set); stdout: {}",
        stdout(&out)
    );
}
