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
async fn outcome_unmap_is_pm_gated_off_the_cli_dev_refused() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let refused = run(&bpmux, "dev", &["outcome", "unmap", "1", "2"]).await;
    assert!(
        !refused.status.success(),
        "outcome unmap is pm-lane; dev must be refused off the CLI (the gated handler, not a generic verb)"
    );
    assert!(
        stderr(&refused).contains("pm"),
        "the refusal names the pm-lane; stderr: {}",
        stderr(&refused)
    );
}

#[tokio::test]
async fn outcome_reorder_with_no_target_is_rejected_by_the_grammar() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run(&bpmux, "pm", &["outcome", "reorder", "1", "2"]).await;
    assert!(
        !out.status.success(),
        "outcome reorder needs --to <P>; with none the grammar rejects it"
    );
    assert!(
        stderr(&out).contains("--to") || stderr(&out).contains("to"),
        "the error names the required --to; stderr: {}",
        stderr(&out)
    );
}
