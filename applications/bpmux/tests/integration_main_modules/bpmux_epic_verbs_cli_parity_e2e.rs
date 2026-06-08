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
        .expect("run bpmux epic verb subprocess")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_epic_set_parent_blocks_a_non_pm_with_the_require_role_error() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["epic", "set-parent", "1", "--epic", "2"]).await;
    assert!(
        !out.status.success(),
        "set_parent_epic is PM-only; dev must be refused"
    );
    assert!(
        stderr(&out).contains("restricted to pm"),
        "bpmux epic set-parent hits the SAME require_role(pm) own-guard as MCP set_parent_epic; dev refused; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_epic_set_parent_owner_bypasses_the_pm_only_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["epic", "set-parent", "1", "--epic", "2"]).await;
    assert!(
        !stderr(&out).contains("restricted to pm"),
        "the owner bypasses set_parent_epic's pm-only require_role guard via the #1709 superset-extension; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_epic_set_parent_owner_reaches_the_handler_past_the_gate() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["epic", "set-parent", "1", "--epic", "2"]).await;
    let e = stderr(&out);
    assert!(
        e.contains("not found") || e.contains("not an Epic"),
        "past the bypassed gate the owner's set-parent reaches set_parent_epic's story-validation (proving the verb hits the SAME handler, not a CLI-side stub); stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_epic_unset_parent_blocks_a_non_pm_with_the_require_role_error() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["epic", "unset-parent", "1"]).await;
    assert!(
        !out.status.success(),
        "unset_parent_epic is PM-only; dev must be refused"
    );
    assert!(
        stderr(&out).contains("restricted to pm"),
        "bpmux epic unset-parent hits the SAME require_role(pm) own-guard as MCP unset_parent_epic; dev refused; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_epic_unset_parent_owner_bypasses_the_pm_only_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["epic", "unset-parent", "1"]).await;
    assert!(
        !stderr(&out).contains("restricted to pm"),
        "the owner bypasses unset_parent_epic's pm-only require_role guard via the #1709 superset-extension; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_epic_unset_parent_succeeds_for_pm_off_mcp() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let child = create_target_story(
        &pm,
        "[Feature] ungroup target",
        "ungrouped from its epic via CLI",
    )
    .await;
    pm.close().await;
    let out = run_verb(&bpmux, "pm", &["epic", "unset-parent", &child.to_string()]).await;
    assert!(
        out.status.success(),
        "pm's bpmux epic unset-parent completes the unset_parent_epic write off MCP; stderr: {}",
        stderr(&out)
    );
    let o = String::from_utf8_lossy(&out.stdout);
    assert!(
        o.contains(&format!("#{child} ungrouped from parent epic")),
        "the CLI prints the handler's success message; stdout: {o}"
    );
}
