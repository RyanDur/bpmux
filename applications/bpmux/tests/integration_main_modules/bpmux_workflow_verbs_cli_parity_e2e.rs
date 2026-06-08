#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use serde_json::json;

async fn run_verb(bpmux: &TestBpmux, role: &str, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux verb subprocess")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_start_blocks_a_non_dev_with_the_same_structured_role_error_as_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "pm",
        &["start", "1", "--paraphrase", "p", "--falsifier", "f"],
    )
    .await;
    assert!(
        !out.status.success(),
        "pickup is dev-only; pm must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("workflow-role-not-allowed")
            && e.contains("\"actor_role\":\"pm\"")
            && e.contains("dev"),
        "bpmux start hits the SAME gated pickup role-guard as MCP start_story (dev-only) — pm is refused with the structured error; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_start_owner_bypasses_the_pickup_role_guard_via_the_reused_superset() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "owner",
        &["start", "1", "--paraphrase", "p", "--falsifier", "f"],
    )
    .await;
    assert!(
        !stderr(&out).contains("workflow-role-not-allowed"),
        "the owner bypasses the dev-only pickup role-guard via the #1548 superset (reused, not re-implemented); any block is a downstream gate; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_deliver_blocks_a_non_dev_with_the_same_structured_role_error_as_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "pm", &["deliver", "1", "--observation", "x"]).await;
    assert!(
        !out.status.success(),
        "deliver is dev-only; pm must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("workflow-role-not-allowed")
            && e.contains("\"actor_role\":\"pm\"")
            && e.contains("dev"),
        "bpmux deliver hits the SAME gated deliver role-guard as MCP deliver_story; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_deliver_owner_bypasses_the_deliver_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["deliver", "1", "--observation", "x"]).await;
    assert!(
        !stderr(&out).contains("workflow-role-not-allowed"),
        "the owner bypasses the dev-only deliver role-guard via the reused superset; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_fail_blocks_a_non_actor_against_the_substrate_role_map_not_the_doc_prose() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["fail", "1", "--reason", "x"]).await;
    assert!(
        !out.status.success(),
        "fail-desk-check excludes dev; dev must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("workflow-role-not-allowed")
            && e.contains("\"actor_role\":\"dev\"")
            && e.contains("pm")
            && e.contains("designer"),
        "bpmux fail hits the SAME gated fail-desk-check role-guard as MCP — and the allowed set is the SUBSTRATE [pm,designer] (bundled_default), not the tool-doc's 'designer-only'; dev is refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_fail_owner_bypasses_the_fail_desk_check_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["fail", "1", "--reason", "x"]).await;
    assert!(
        !stderr(&out).contains("workflow-role-not-allowed"),
        "the owner bypasses the [pm,designer] fail role-guard via the reused superset; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_reject_blocks_a_non_pm_with_the_same_structured_role_error_as_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["reject", "1", "--reason", "x"]).await;
    assert!(
        !out.status.success(),
        "reject is pm-only; dev must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("workflow-role-not-allowed")
            && e.contains("\"actor_role\":\"dev\"")
            && e.contains("pm"),
        "bpmux reject hits the SAME gated reject role-guard as MCP reject_story (pm-only); dev is refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_reject_owner_bypasses_the_reject_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["reject", "1", "--reason", "x"]).await;
    assert!(
        !stderr(&out).contains("workflow-role-not-allowed"),
        "the owner bypasses the pm-only reject role-guard via the reused superset; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_start_then_deliver_drive_the_dev_lifecycle_off_mcp() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let number = create_target_story(
        &pm,
        "[Feature] cli-dev-lifecycle-target",
        "dev drives start then deliver via the CLI subprocess, off MCP",
    )
    .await;
    pm.call_tool(
        "move_story",
        json!({"number": number, "target_state": "ready", "rationale": "promote icebox->ready for the cli start positive-control"}),
    )
    .await
    .expect("move to ready");
    pm.close().await;
    let started = run_verb(
        &bpmux,
        "dev",
        &[
            "start",
            &number.to_string(),
            "--paraphrase",
            "cli pickup paraphrase",
            "--falsifier",
            "cli falsifier reframing",
        ],
    )
    .await;
    assert!(
        started.status.success(),
        "dev's bpmux start on a ready story completes the pickup write off MCP; stderr: {}",
        stderr(&started)
    );
    let delivered = run_verb(
        &bpmux,
        "dev",
        &[
            "deliver",
            &number.to_string(),
            "--observation",
            "drove start+deliver end-to-end via the bpmux CLI subprocess",
        ],
    )
    .await;
    assert!(
        delivered.status.success(),
        "dev's bpmux deliver on the in-progress story completes the deliver write off MCP; stderr: {}",
        stderr(&delivered)
    );
    let out = String::from_utf8_lossy(&delivered.stdout);
    assert!(
        out.contains(&format!("#{number}")),
        "the CLI prints the handler's own success message — the gated writes completed end-to-end; stdout: {out}"
    );
}
