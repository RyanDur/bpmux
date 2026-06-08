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
        .expect("run bpmux arrangement verb subprocess")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_move_blocks_a_non_pm_with_the_same_structured_role_error_as_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "dev",
        &[
            "move",
            "1",
            "--target-state",
            "ready",
            "--rationale",
            "cli move test",
        ],
    )
    .await;
    assert!(
        !out.status.success(),
        "move is PM-only for all moves; dev must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("workflow-role-not-allowed")
            && e.contains("\"actor_role\":\"dev\"")
            && e.contains("pm"),
        "bpmux move hits the SAME gated move-with-position role-guard as MCP move_story — PM-only for ALL moves (substrate [pm], not the prose's any-role-column); dev refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_move_owner_bypasses_the_pm_only_move_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "owner",
        &[
            "move",
            "1",
            "--target-state",
            "ready",
            "--rationale",
            "cli move test",
        ],
    )
    .await;
    assert!(
        !stderr(&out).contains("workflow-role-not-allowed"),
        "the owner bypasses the PM-only move role-guard via the #1548 superset (enforce_role_allowed); stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_move_succeeds_for_pm_off_mcp() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let number = create_target_story(
        &pm,
        "[Feature] cli-move-target",
        "pm moves it to ready via the bpmux CLI, off MCP",
    )
    .await;
    pm.close().await;
    let out = run_verb(
        &bpmux,
        "pm",
        &[
            "move",
            &number.to_string(),
            "--target-state",
            "ready",
            "--rationale",
            "promote icebox->ready via bpmux move",
        ],
    )
    .await;
    assert!(
        out.status.success(),
        "pm's bpmux move completes the move write off MCP; stderr: {}",
        stderr(&out)
    );
    let o = String::from_utf8_lossy(&out.stdout);
    assert!(
        o.contains(&format!("#{number}")),
        "the CLI prints the handler's success message — the gated move completed; stdout: {o}"
    );
}

#[tokio::test]
async fn bpmux_label_blocks_a_non_pm_via_the_require_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["label", "1", "--label", "awaiting team"]).await;
    assert!(
        !out.status.success(),
        "label is PM-only (require_role); dev must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("restricted to pm"),
        "bpmux label hits the SAME require_role(pm) own-guard as MCP label_story; dev refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_label_owner_bypasses_the_pm_only_require_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["label", "1", "--label", "awaiting team"]).await;
    assert!(
        !stderr(&out).contains("restricted to pm"),
        "the owner bypasses the PM-only label require_role guard via the #1709 require_role superset-extension; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_label_succeeds_for_pm_off_mcp() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let number = create_target_story(
        &pm,
        "[Feature] cli-label-target",
        "pm labels it via the bpmux CLI, off MCP",
    )
    .await;
    pm.close().await;
    let out = run_verb(
        &bpmux,
        "pm",
        &["label", &number.to_string(), "--label", "awaiting team"],
    )
    .await;
    assert!(
        out.status.success(),
        "pm's bpmux label completes the label write off MCP; stderr: {}",
        stderr(&out)
    );
    let o = String::from_utf8_lossy(&out.stdout);
    assert!(
        o.contains(&format!("#{number}")),
        "the CLI prints the handler's success message — the gated label completed; stdout: {o}"
    );
}
