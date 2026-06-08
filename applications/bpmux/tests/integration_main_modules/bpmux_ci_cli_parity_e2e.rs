#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::pty::TestBpmux;

async fn run_verb(bpmux: &TestBpmux, role: &str, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux ci verb subprocess")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_ci_state_reaches_the_bearer_auth_endpoint_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["ci", "state"]).await;
    assert!(
        out.status.success(),
        "bpmux ci state hits the same Bearer-auth /api/ci endpoint the MCP get_ci_state tool uses (a 401 would fail); stderr: {}",
        stderr(&out)
    );
    assert!(
        !stdout(&out).trim().is_empty(),
        "ci state returns the CI-state contract off MCP (non-vacuous: a real payload, not empty / a route-404); stdout: {}",
        stdout(&out)
    );
}
