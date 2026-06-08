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
async fn bpmux_iteration_close_reaches_the_handler_with_no_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["iteration", "close", "--iteration", "1"]).await;
    let e = stderr(&out);
    assert!(
        e.contains("not found") || e.contains("not an Iteration"),
        "bpmux iteration close reaches the SAME gated /api/close_iteration handler (no role-guard, like MCP close_iteration) — iteration #1 absent yields the handler's iteration-validation error, not a route-404 nor a CLI-added role-guard; stderr: {e}"
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
