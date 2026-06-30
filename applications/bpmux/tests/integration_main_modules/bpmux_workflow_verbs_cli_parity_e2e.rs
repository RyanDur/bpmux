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
    let id = create_target_story(
        &pm,
        "[Feature] cli-dev-lifecycle-target",
        "dev drives start then deliver via the CLI subprocess, off MCP",
    )
    .await;
    pm.call_tool(
        "move_story",
        json!({"id": id, "target_state": "ready", "rationale": "promote icebox->ready for the cli start positive-control"}),
    )
    .await
    .expect("move to ready");
    pm.close().await;
    let started = run_verb(
        &bpmux,
        "dev",
        &[
            "start",
            &id.to_string(),
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
            &id.to_string(),
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
        out.contains(&format!("#{id}")),
        "the CLI prints the handler's own success message — the gated writes completed end-to-end; stdout: {out}"
    );
}

#[tokio::test]
async fn bpmux_verdict_blocks_a_non_evidence_role_via_its_own_handler_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "pm",
        &[
            "verdict",
            "1",
            "--verdict",
            "verified",
            "--grounding",
            "cli cite",
        ],
    )
    .await;
    assert!(
        !out.status.success(),
        "submit_verdict is dev/designer-only; pm must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("restricted to dev or designer") && e.contains("pm"),
        "bpmux verdict hits the SAME submit_verdict handler-guard as MCP (its own dev/designer check, not a transition role-guard); pm is refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_verdict_does_not_owner_bypass_because_it_is_not_a_transition_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "owner",
        &[
            "verdict",
            "1",
            "--verdict",
            "verified",
            "--grounding",
            "cli cite",
        ],
    )
    .await;
    let e = stderr(&out);
    assert!(
        e.contains("restricted to dev or designer"),
        "the owner is NOT an evidence role and is refused for verdict — the #1548 superset-bypass lives in enforce_role_allowed (transitions); submit_verdict uses its OWN guard, so the owner does not bypass it (identical to MCP). This is why per-verb grounding beats per-story prose; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_verdict_succeeds_for_a_dev_evidence_role_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "dev",
        &[
            "verdict",
            "1",
            "--verdict",
            "verified",
            "--grounding",
            "applications/bpmux/src/cli/write.rs run_verdict",
        ],
    )
    .await;
    assert!(
        out.status.success(),
        "dev is an evidence role — its bpmux verdict records off MCP; stderr: {}",
        stderr(&out)
    );
    let o = String::from_utf8_lossy(&out.stdout);
    assert!(
        o.contains("verdict recorded") && o.contains("#1"),
        "the CLI prints the handler's success message — the verdict write completed; stdout: {o}"
    );
}

#[tokio::test]
async fn bpmux_accept_blocks_a_non_acceptor_proving_it_routes_through_the_gated_handler() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["accept", "1"]).await;
    assert!(
        !out.status.success(),
        "team-verifiable accept is pm-only; dev must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("workflow-role-not-allowed")
            && e.contains("\"actor_role\":\"dev\"")
            && e.contains("pm"),
        "bpmux accept routes through the GATED handle_accept_story (the structured role-error proves it) — NOT the gate-skipping post_accept_story-direct REST (which has no role-guard); dev is refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_accept_owner_bypasses_the_acceptor_role_guard_via_the_reused_superset() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["accept", "1"]).await;
    assert!(
        !stderr(&out).contains("workflow-role-not-allowed"),
        "the owner bypasses the pm-only acceptor role-guard via the reused #1548 superset; any block is a downstream gate (evidence/CI); stderr: {}",
        stderr(&out)
    );
}
