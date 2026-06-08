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
        .expect("run bpmux creation verb subprocess")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn create_args() -> Vec<&'static str> {
    vec![
        "create",
        "--title",
        "a regular non-chore story",
        "--user-job",
        "as a role, I want X so that Y",
        "--observation-falsifier",
        "reject if X",
    ]
}

#[tokio::test]
async fn bpmux_create_blocks_a_non_pm_non_owner_for_a_non_chore_via_the_custom_pm_lane_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &create_args()).await;
    assert!(
        !out.status.success(),
        "create_story is pm-lane (dev only for [Chore]); a dev non-chore create must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("pm-lane"),
        "bpmux create hits the SAME custom pm-lane guard as MCP create_story; dev (non-chore) refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_create_owner_bypasses_the_custom_pm_lane_guard_via_the_1710_superset() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &create_args()).await;
    assert!(
        !stderr(&out).contains("pm-lane"),
        "the owner bypasses create_story's CUSTOM pm-lane guard via the #1710 owner-superset extension (the #1709 require_role extension does NOT reach create's custom check); stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_create_allows_dev_for_a_chore_class_story_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "dev",
        &[
            "create",
            "--title",
            "[Chore] tidy a module",
            "--user-job",
            "as dev, the module is decomposed",
            "--observation-falsifier",
            "reject if it still exceeds the limit",
        ],
    )
    .await;
    assert!(
        out.status.success(),
        "dev authoring a [Chore]-class story is the chore-ownership lane — it completes off MCP; stderr: {}",
        stderr(&out)
    );
    let o = String::from_utf8_lossy(&out.stdout);
    assert!(
        o.contains("story created"),
        "the CLI prints the handler's success message; stdout: {o}"
    );
}

#[tokio::test]
async fn bpmux_create_succeeds_for_pm_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "pm", &create_args()).await;
    assert!(
        out.status.success(),
        "pm's bpmux create completes the create write off MCP; stderr: {}",
        stderr(&out)
    );
    let o = String::from_utf8_lossy(&out.stdout);
    assert!(
        o.contains("story created"),
        "the CLI prints the handler's success message; stdout: {o}"
    );
}

#[tokio::test]
async fn bpmux_edit_blocks_a_non_pm_via_the_require_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["edit", "1", "--title", "new title"]).await;
    assert!(
        !out.status.success(),
        "edit is PM-only; dev must be refused"
    );
    assert!(
        stderr(&out).contains("restricted to pm"),
        "bpmux edit hits the SAME require_role(pm) own-guard as MCP edit_story; dev refused; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_edit_owner_bypasses_the_pm_only_require_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "owner", &["edit", "1", "--title", "new title"]).await;
    assert!(
        !stderr(&out).contains("restricted to pm"),
        "the owner bypasses the pm-only edit require_role guard via the #1709 require_role superset-extension; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_edit_succeeds_for_pm_off_mcp() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let number =
        create_target_story(&pm, "[Feature] cli-edit-target", "pm edits it via the CLI").await;
    pm.close().await;
    let out = run_verb(
        &bpmux,
        "pm",
        &[
            "edit",
            &number.to_string(),
            "--title",
            "[Feature] cli-edit-target (edited)",
        ],
    )
    .await;
    assert!(
        out.status.success(),
        "pm's bpmux edit completes the edit write off MCP; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_comment_allows_any_role_off_mcp_no_role_restriction() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let number = create_target_story(
        &pm,
        "[Feature] cli-comment-target",
        "dev comments on it via the CLI",
    )
    .await;
    pm.close().await;
    let out = run_verb(
        &bpmux,
        "dev",
        &[
            "comment",
            &number.to_string(),
            "--body",
            "a dev comment via the bpmux CLI",
        ],
    )
    .await;
    assert!(
        out.status.success(),
        "comment_on_story has NO role-restriction — a non-pm (dev) comments off MCP and it completes; stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_close_blocks_a_non_pm_with_the_same_structured_role_error_as_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "dev",
        &["close", "1", "--reason", "cli close test reason"],
    )
    .await;
    assert!(
        !out.status.success(),
        "close is PM-only (close transition); dev must be refused"
    );
    let e = stderr(&out);
    assert!(
        e.contains("workflow-role-not-allowed")
            && e.contains("\"actor_role\":\"dev\"")
            && e.contains("pm"),
        "bpmux close hits the SAME gated close transition role-guard as MCP close_story (PM-only); dev refused; stderr: {e}"
    );
}

#[tokio::test]
async fn bpmux_close_owner_bypasses_the_close_transition_role_guard() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(
        &bpmux,
        "owner",
        &["close", "1", "--reason", "cli close test reason"],
    )
    .await;
    assert!(
        !stderr(&out).contains("workflow-role-not-allowed"),
        "the owner bypasses the close transition role-guard via the #1548 superset (enforce_role_allowed); stderr: {}",
        stderr(&out)
    );
}

#[tokio::test]
async fn bpmux_close_succeeds_for_pm_off_mcp() {
    let bpmux = TestBpmux::start();
    let pm = TestBridge::connect(&bpmux, "pm").await;
    let number = create_target_story(
        &pm,
        "[Feature] cli-close-target",
        "pm closes it via the CLI",
    )
    .await;
    pm.close().await;
    let out = run_verb(
        &bpmux,
        "pm",
        &[
            "close",
            &number.to_string(),
            "--reason",
            "closing via the bpmux CLI off MCP",
        ],
    )
    .await;
    assert!(
        out.status.success(),
        "pm's bpmux close completes the close write off MCP; stderr: {}",
        stderr(&out)
    );
}
