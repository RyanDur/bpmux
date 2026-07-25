#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, seed_team_verifiable_evidence};

async fn run_pass(bpmux: &TestBpmux, role: &str, id: u64) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(["pass", &id.to_string(), "--acceptor", "team"])
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux pass subprocess")
}

#[tokio::test]
async fn bpmux_pass_blocks_a_non_actor_role_with_the_same_structured_role_error_as_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_pass(&bpmux, "dev", 1).await;
    assert!(
        !out.status.success(),
        "dev is not the pass-desk-check actor (designer-only); the CLI must be substrate-refused off MCP"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("workflow-role-not-allowed"),
        "the CLI hits the SAME gated /api/rpc/pass_desk_check handler as the bridge — dev's block is the structured role-gate, not a routing/auth miss; stderr: {stderr}"
    );
    assert!(
        crate::harness::story_helpers::structured_error(&stderr)["actor_role"] == "dev"
            && stderr.contains("designer"),
        "the structured error names dev as the firing role and designer as the only allowed actor — identical to the MCP path; stderr: {stderr}"
    );
}

#[tokio::test]
async fn bpmux_pass_for_the_designer_actor_without_evidence_is_blocked_at_the_evidence_gate() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_pass(&bpmux, "designer", 1).await;
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("workflow-role-not-allowed"),
        "designer IS the pass actor — it clears the role-guard via the CLI; any block is a downstream gate, never the role-gate; stderr: {stderr}"
    );
    assert!(
        stderr.contains("designer-verdict-missing"),
        "with no fresh designer verdict the evidence gate fires via the CLI exactly as via MCP — proving the whole gate-chain, not just the role-guard, runs on the shared endpoint; stderr: {stderr}"
    );
}

#[tokio::test]
async fn bpmux_pass_succeeds_for_the_designer_actor_with_fresh_evidence_and_green_ci() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let id = create_target_story(
        &pm,
        "[Feature] cli-pass-happy-target",
        "the designer's bpmux pass must carry it through the gate to delivered",
    )
    .await;
    pm.close().await;
    seed_team_verifiable_evidence(&bpmux, id).await;
    let out = run_pass(&bpmux, "designer", id).await;
    assert!(
        out.status.success(),
        "designer (the pass actor) with a fresh designer-verdict and green CI must pass the desk-check off MCP; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("#{id}")) && stdout.contains("passed desk-check"),
        "the CLI prints the handler's own success message — the gated write completed end-to-end through the shared endpoint; stdout: {stdout}"
    );
}

#[tokio::test]
async fn bpmux_pass_lets_the_owner_clear_the_designer_only_role_guard_via_the_superset() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let id = create_target_story(
        &pm,
        "[Feature] cli-pass-owner-superset-target",
        "the owner's bpmux pass must clear the designer-only role-guard via the superset-bypass",
    )
    .await;
    pm.close().await;
    seed_team_verifiable_evidence(&bpmux, id).await;
    let out = run_pass(&bpmux, "owner", id).await;
    assert!(
        out.status.success(),
        "the owner is a superset of every role (#1548): the owner's bpmux pass clears the SAME designer-only role-guard that refuses dev, and completes the desk-check — the superset-bypass reaches the CLI/RPC path, not just the handler; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("#{id}")) && stdout.contains("passed desk-check"),
        "the owner's gated write completes end-to-end off MCP; stdout: {stdout}"
    );
}
