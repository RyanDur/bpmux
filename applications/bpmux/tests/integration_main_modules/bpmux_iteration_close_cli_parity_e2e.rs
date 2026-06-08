#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::extract_story_number;

async fn run_verb(bpmux: &TestBpmux, role: &str, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux iteration close subprocess")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_iteration_close_blocks_a_non_pm_with_the_require_role_error() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["iteration", "close", "--iteration", "1"]).await;
    assert!(
        !out.status.success(),
        "close_iteration is PM-only; dev must be refused"
    );
    assert!(
        stderr(&out).contains("restricted to pm"),
        "bpmux iteration close routes through the GATED /api/rpc/close_iteration (require_purpose pm), mirroring the MCP close_iteration tool's gate — dev refused, NOT the old bare gate-skipping /api/close_iteration REST; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_iteration_close_owner_bypasses_the_pm_only_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["iteration", "close", "--iteration", "1"]).await;
    assert!(
        !stderr(&out).contains("restricted to pm"),
        "the owner bypasses close_iteration's pm-only require_role guard via the #1709 superset (then hits iteration-not-found, not the role-gate); stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_iteration_close_succeeds_for_pm_off_mcp() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let create = pm
        .call_tool(
            "create_iteration",
            serde_json::json!({ "theme_name": "Telescopium" }),
        )
        .await
        .expect("create_iteration should succeed");
    pm.close().await;
    let number = extract_story_number(create.as_str().unwrap_or(""));
    let out = run_verb(
        &bpmux,
        "pm",
        &["iteration", "close", "--iteration", &number.to_string()],
    )
    .await;
    assert!(
        out.status.success(),
        "pm's bpmux iteration close completes the close_iteration write off MCP (ready queue empty); stderr: {}",
        stderr(&out)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("closed"),
        "the CLI confirms the close; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
