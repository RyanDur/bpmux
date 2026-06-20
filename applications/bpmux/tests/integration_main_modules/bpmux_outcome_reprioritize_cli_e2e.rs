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

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn outcome_number(create_stdout: &str) -> u64 {
    let digits: String = create_stdout
        .split('#')
        .nth(1)
        .unwrap_or_default()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().expect("outcome number in create output")
}

#[tokio::test]
async fn reprioritize_is_pm_gated_off_the_cli_dev_refused() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let created = run(
        &bpmux,
        "pm",
        &["outcome", "create", "--title", "X", "--horizon", "next"],
    )
    .await;
    let number = outcome_number(&stdout(&created));

    let refused = run(
        &bpmux,
        "dev",
        &[
            "outcome",
            "reprioritize",
            &number.to_string(),
            "--to",
            "now",
        ],
    )
    .await;
    assert!(
        !refused.status.success(),
        "reprioritize is pm-lane; dev must be refused off the CLI (the gated handler, not a thin alias)"
    );
    assert!(
        stderr(&refused).contains("pm"),
        "the refusal names the pm-lane; stderr: {}",
        stderr(&refused)
    );
}
