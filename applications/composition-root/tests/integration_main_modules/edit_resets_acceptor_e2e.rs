#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::pty::TestBpmux;

async fn run_create(bpmux: &TestBpmux, role: &str, user_job: &str) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args([
            "create",
            "--title",
            "[Chore] edit-acceptor target",
            "--user-job",
            user_job,
            "--observation-falsifier",
            "reject if the --acceptor flag cannot re-set the acceptor off MCP",
        ])
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux create subprocess")
}

async fn run_edit_acceptor(
    bpmux: &TestBpmux,
    role: &str,
    id: u64,
    acceptor: &str,
) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(["edit", &id.to_string(), "--acceptor", acceptor])
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux edit subprocess")
}

fn created_number(out: &std::process::Output) -> u64 {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let digits: String = stdout
        .split('#')
        .nth(1)
        .unwrap_or("")
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("could not parse created story number from stdout: {stdout}"))
}

#[tokio::test]
async fn off_mcp_edit_acceptor_flag_reaches_the_gated_edit_handler() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();

    let created = run_create(
        &bpmux,
        "pm",
        "As a dev, I want the module decomposed so the file stays under the size gate",
    )
    .await;
    assert!(
        created.status.success(),
        "create completes off MCP; stderr: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    let id = created_number(&created);

    let edited = run_edit_acceptor(&bpmux, "pm", id, "owner").await;
    assert!(
        edited.status.success(),
        "an acceptor-only edit (no --title/--body) is accepted and reaches the handler; stderr: {}",
        String::from_utf8_lossy(&edited.stderr)
    );
    let stdout = String::from_utf8_lossy(&edited.stdout);
    assert!(
        stdout.contains("acceptor"),
        "the edit reports the acceptor field was updated; stdout: {stdout}"
    );
}

#[tokio::test]
async fn off_mcp_edit_with_an_invalid_acceptor_value_fails_loud() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();

    let created = run_create(
        &bpmux,
        "pm",
        "As a dev, I want the module decomposed so the file stays under the size gate",
    )
    .await;
    assert!(created.status.success(), "create completes off MCP");
    let id = created_number(&created);

    let edited = run_edit_acceptor(&bpmux, "pm", id, "onwer").await;
    assert!(
        !edited.status.success(),
        "a typo'd --acceptor must fail loud, not silently leave the acceptor unchanged"
    );
    let stderr = String::from_utf8_lossy(&edited.stderr);
    assert!(
        stderr.contains("team") && stderr.contains("owner") && stderr.contains("dev"),
        "the error names the allowed acceptor classes so the typo is obvious; got stderr:\n{stderr}"
    );
}
