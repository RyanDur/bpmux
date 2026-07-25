#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness::pty::TestBpmux;

async fn run(bpmux: &TestBpmux, role: &str, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux subprocess")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn iteration_remove_member_is_pm_gated_off_the_cli_dev_refused() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let refused = run(
        &bpmux,
        "dev",
        &["iteration", "remove-member", "1", "2", "--reason", "r"],
    )
    .await;
    assert!(
        !refused.status.success(),
        "remove-member is pm-lane; dev must be refused off the CLI"
    );
    assert!(
        stderr(&refused).contains("pm"),
        "the refusal names the pm-lane; stderr: {}",
        stderr(&refused)
    );
}

#[tokio::test]
async fn iteration_remove_member_with_no_reason_is_rejected_by_the_grammar() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run(&bpmux, "pm", &["iteration", "remove-member", "1", "2"]).await;
    assert!(
        !out.status.success(),
        "remove-member is destructive; --reason is required (the audit-trail)"
    );
    assert!(
        stderr(&out).contains("reason"),
        "the error names the required --reason; stderr: {}",
        stderr(&out)
    );
}
