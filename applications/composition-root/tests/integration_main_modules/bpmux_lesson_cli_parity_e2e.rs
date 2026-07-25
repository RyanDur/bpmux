#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::pty::TestBpmux;

async fn run_verb(bpmux: &TestBpmux, role: &str, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux lesson verb subprocess")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_lesson_save_then_list_round_trips_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();

    let save = run_verb(
        &bpmux,
        "dev",
        &[
            "lesson",
            "save",
            "--topic",
            "cli-roundtrip",
            "--lesson",
            "a lesson saved via the bpmux CLI",
        ],
    )
    .await;
    assert!(
        save.status.success(),
        "bpmux lesson save persists via the same ungated /api/lessons POST the bridge uses; stderr: {}",
        stderr(&save)
    );

    let list = run_verb(
        &bpmux,
        "dev",
        &["lesson", "list", "--topic", "cli-roundtrip"],
    )
    .await;
    assert!(
        list.status.success(),
        "bpmux lesson list reads via the same /api/lessons GET; stderr: {}",
        stderr(&list)
    );
    assert!(
        stdout(&list).contains("a lesson saved via the bpmux CLI"),
        "the round-trip proves the CLI save+list hit the real /api/lessons store off MCP (non-vacuous: the saved lesson reads back); stdout: {}",
        stdout(&list)
    );
}

#[tokio::test]
async fn bpmux_lesson_list_peer_reaches_the_endpoint_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["lesson", "list-peer"]).await;
    assert!(
        out.status.success(),
        "bpmux lesson list-peer reaches the same /api/peer-lessons GET the bridge uses (empty peer-set is a valid 200, not a route-404); stderr: {}",
        stderr(&out)
    );
}
