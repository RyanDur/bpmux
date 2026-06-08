#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;

async fn run_verb(bpmux: &TestBpmux, role: &str, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux read verb subprocess")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_backlog_reaches_the_gated_rpc_read_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["backlog"]).await;
    assert!(
        out.status.success(),
        "bpmux backlog reaches the same gated /api/rpc/view_backlog read the bridge uses (a 401 would fail); stderr: {}",
        stderr(&out)
    );
    assert!(
        !stdout(&out).trim().is_empty(),
        "backlog returns the view_backlog contract off MCP (non-vacuous: a real payload, not empty / a route-404); stdout: {}",
        stdout(&out)
    );
}

#[tokio::test]
async fn bpmux_timeline_reaches_the_gated_rpc_read_for_a_real_story_off_mcp() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let number = create_target_story(&pm, "[Feature] timeline target", "read its timeline").await;
    pm.close().await;
    let out = run_verb(&bpmux, "dev", &["timeline", &number.to_string()]).await;
    assert!(
        out.status.success(),
        "bpmux timeline reaches /api/rpc/view_story_timeline for a real story off MCP; stderr: {}",
        stderr(&out)
    );
    assert!(
        !stdout(&out).trim().is_empty(),
        "timeline returns the contract off MCP (shares the dispatch_write rpc-read path with backlog/comments/verdicts/epic-children); stdout: {}",
        stdout(&out)
    );
}
