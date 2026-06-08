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
        .expect("run bpmux iteration add subprocess")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn add_args() -> Vec<&'static str> {
    vec![
        "iteration",
        "add",
        "--iteration",
        "1",
        "--story",
        "2",
        "--position",
        "1",
    ]
}

#[tokio::test]
async fn bpmux_iteration_add_blocks_a_non_pm_with_the_workflow_role_error() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &add_args()).await;
    assert!(
        !out.status.success(),
        "add-to-iteration is PM-only; dev must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("workflow-role-not-allowed") && e.contains("\"actor_role\":\"dev\""),
        "bpmux iteration add hits the SAME gated add-to-iteration transition guard as MCP add_story_to_iteration; dev refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_iteration_add_owner_bypasses_the_transition_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &add_args()).await;
    assert!(
        !stderr(&out).contains("workflow-role-not-allowed"),
        "the owner bypasses the add-to-iteration transition guard via the #1548 superset; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_iteration_add_owner_reaches_the_handler_past_the_gate() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &add_args()).await;
    let e = stderr(&out);
    assert!(
        e.contains("not found") || e.contains("not an Iteration"),
        "past the bypassed gate the owner's add reaches add_story_to_iteration's story-validation (the SAME handler, not a CLI-side stub); stderr: {e}"
    );
}
