#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;

async fn run_create(bpmux: &TestBpmux, role: &str, user_job: &str) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args([
            "create",
            "--title",
            "[Chore] acceptor-derivation target",
            "--user-job",
            user_job,
            "--observation-falsifier",
            "reject if the acceptor is None on the board",
        ])
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux create subprocess")
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

async fn story_text(reader: &TestBridge, number: u64) -> String {
    let story = reader
        .call_tool("get_story", serde_json::json!({ "number": number }))
        .await
        .expect("get_story should succeed");
    story
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| story.to_string())
}

#[tokio::test]
async fn off_mcp_create_derives_the_dev_acceptor_from_a_dev_use_axis() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let reader = TestBridge::connect(&bpmux, "pm").await;

    let out = run_create(
        &bpmux,
        "dev",
        "As a dev, I want the module decomposed so the file stays under the size gate",
    )
    .await;
    assert!(
        out.status.success(),
        "dev [Chore] create completes off MCP; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = story_text(&reader, created_number(&out)).await;
    assert!(
        text.contains("awaiting dev"),
        "a 'As a dev' use-axis DERIVES the dev acceptor at create — the board shows awaiting dev before desk-check, not None (the acceptor-axis canon: who-USES-the-feature, not the title-prefix); got:\n{text}"
    );

    reader.close().await;
}

#[tokio::test]
async fn off_mcp_create_derives_the_owner_acceptor_from_an_owner_use_axis() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let reader = TestBridge::connect(&bpmux, "pm").await;

    let out = run_create(
        &bpmux,
        "dev",
        "As the owner, I want the roadmap legible so I can see progress at a glance",
    )
    .await;
    assert!(out.status.success(), "create completes off MCP");

    let text = story_text(&reader, created_number(&out)).await;
    assert!(
        text.contains("awaiting owner"),
        "an 'As the owner' use-axis DERIVES the owner acceptor — the use-axis determines the acceptor independent of the [Chore] story-type; got:\n{text}"
    );
    assert!(
        !text.contains("awaiting dev") && !text.contains("awaiting team"),
        "single-acceptor-label invariant — owner-axis must not also carry dev/team; got:\n{text}"
    );

    reader.close().await;
}

#[tokio::test]
async fn off_mcp_create_derives_the_team_acceptor_from_an_agent_use_axis() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let reader = TestBridge::connect(&bpmux, "pm").await;

    let out = run_create(
        &bpmux,
        "dev",
        "As an agent on the team, I want shared introspection so we all read the same truth",
    )
    .await;
    assert!(out.status.success(), "create completes off MCP");

    let text = story_text(&reader, created_number(&out)).await;
    assert!(
        text.contains("awaiting team"),
        "a multi-agent / team use-axis DERIVES the team acceptor; got:\n{text}"
    );

    reader.close().await;
}
