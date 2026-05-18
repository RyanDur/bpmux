//! Shared helpers for agent-side E2E tests that need a target story.
//!
//! Most #476 Phase 1 backfill tests follow the pattern:
//!
//!   1. PM calls create_story → message contains "#N"
//!   2. Test extracts N and uses it for subsequent calls
//!
//! Centralising the extract + create-target boilerplate so per-tool
//! tests stay focused on the falsifiers specific to that tool.

use super::bridge::TestBridge;

pub fn extract_story_number(create_msg: &str) -> u64 {
    let tail = &create_msg[create_msg.find('#').expect("create message names #N") + 1..];
    let end = tail
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(tail.len());
    tail[..end].parse().expect("parse story number")
}

/// Seed designer + dev verdict-messages in the channel so the #1385
/// workflow-gate's evidence-check (TeamVerifiable class on `accept`)
/// passes for the test under exercise. Used by tests that fire
/// accept_story under the bundled-default workflow (which requires
/// both verdicts present within evidence_window_seconds).
///
/// The shape "any recent message from required role within window"
/// is slice 3's permissive baseline; future slices may tighten to
/// structured verdict-substrate.
pub async fn seed_team_verifiable_evidence(bpmux: &super::pty::TestBpmux) {
    let designer = TestBridge::connect(bpmux, "designer").await;
    let dev = TestBridge::connect(bpmux, "dev").await;
    designer.send("verdict: substrate verified").await;
    dev.send("verdict: substrate verified").await;
    designer.close().await;
    dev.close().await;
}

pub async fn seed_team_verifiable_evidence_with_mock_gh(
    bpmux: &super::pty::TestBpmux,
    mock_gh_base_url: &str,
) {
    let designer = TestBridge::connect_with_mock_gh(bpmux, "designer", mock_gh_base_url).await;
    let dev = TestBridge::connect_with_mock_gh(bpmux, "dev", mock_gh_base_url).await;
    designer.send("verdict: substrate verified").await;
    dev.send("verdict: substrate verified").await;
    designer.close().await;
    dev.close().await;
}

pub async fn create_target_story(pm: &TestBridge, title: &str, falsifier: &str) -> u64 {
    // #537 default-include `acceptance_test_path` pointing to this
    // harness file (always exists in the workspace tree) so #521's
    // start_story ATP-validator doesn't 400 on E2E targets created
    // via this helper. Use an absolute path resolved from
    // CARGO_MANIFEST_DIR so the validator's cwd-resolution succeeds
    // regardless of where the bridge subprocess inherits its cwd
    // from (PTY-spawned bpmux may run from a different cwd than the
    // cargo test binary).
    let atp = format!(
        "{}/tests/harness/story_helpers.rs",
        env!("CARGO_MANIFEST_DIR")
    );
    let create = pm
        .call_tool(
            "create_story",
            serde_json::json!({
                "title": title,
                "user_job": "When testing affordance E2E, dev needs a target story.",
                "observation_falsifier": falsifier,
                "acceptance_test_path": atp
            }),
        )
        .await
        .expect("create_story should succeed");
    let create_msg = create.as_str().unwrap_or("").to_string();
    extract_story_number(&create_msg)
}
