use crate::harness::mock_gh::issue_json;
use crate::harness::pty::TestBpmux;
use bpmux::integrations::github::relationships::{IterationRelationships, Relationships};
use bpmux::product_management::story::StoryId;
use httpmock::{Method, MockServer};
use serde_json::Value;

fn seeded_bpmux() -> (TestBpmux, MockServer) {
    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(1234, "[Feature] test ready story", &["ready"], None);
    let iteration_body = IterationRelationships {
        outcome: None,
        stories: vec![StoryId::new(1234)],
    }
    .write_into_body("## Iteration Vela\n");
    let iteration_issue = issue_json(
        900,
        "Iteration Vela",
        &["iteration", "current-iteration"],
        Some(&iteration_body),
    );
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue, iteration_issue]).to_string());
    });
    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    (bpmux, server)
}

async fn run_bpmux(port: u16, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .output()
        .await
        .expect("run bpmux read-verb")
}

#[tokio::test]
async fn bpmux_ready_renders_a_compact_index_off_mcp_not_a_flood() {
    let (bpmux, _server) = seeded_bpmux();
    let out = run_bpmux(bpmux.port(), &["ready"]).await;
    assert!(
        out.status.success(),
        "bpmux ready should exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("ready ("),
        "the default render is a compact counts-header index, not a raw dump; stdout: {stdout}"
    );
    assert!(
        stdout.contains("#1234") && stdout.contains("[Feature] test ready story"),
        "the seeded ready story renders as a one-line index entry, sourced from the store off MCP; stdout: {stdout}"
    );
    assert!(
        stdout.len() < 4096,
        "compact-index-not-dump: a single-story ready render must be tiny (vs the 132KB view_board flood); len={}",
        stdout.len()
    );
}

#[tokio::test]
async fn bpmux_ready_json_mode_is_machine_parseable() {
    let (bpmux, _server) = seeded_bpmux();
    let out = run_bpmux(bpmux.port(), &["ready", "--json"]).await;
    assert!(out.status.success(), "bpmux ready --json should exit 0");
    let parsed: Value = serde_json::from_slice(&out.stdout)
        .expect("bpmux ready --json must emit parseable JSON for agent column-free parsing");
    let arr = parsed.as_array().expect("ready --json is a JSON array");
    assert!(
        arr.iter().any(|s| s["id"].as_u64() == Some(1234)),
        "the --json array carries the structured ready stories; got: {parsed}"
    );
}

#[tokio::test]
async fn bpmux_story_renders_a_compact_summary_with_a_full_hint() {
    let (bpmux, _server) = seeded_bpmux();
    let out = run_bpmux(bpmux.port(), &["story", "1234"]).await;
    assert!(out.status.success(), "bpmux story <n> should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("#1234 [Feature] test ready story"),
        "bpmux story renders a compact summary line; stdout: {stdout}"
    );
    assert!(
        stdout.contains("use --full for the body"),
        "the compact summary hints --full for the body rather than dumping it; stdout: {stdout}"
    );
}

#[tokio::test]
async fn bpmux_story_missing_number_exits_nonzero_to_stderr() {
    let (bpmux, _server) = seeded_bpmux();
    let out = run_bpmux(bpmux.port(), &["story", "9999"]).await;
    assert!(
        !out.status.success(),
        "a not-found story must exit non-zero (clig-hygiene: crash-early at the boundary, not silently empty-render)"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("9999"),
        "the not-found diagnostic goes to stderr naming the number; stderr: {stderr}"
    );
}

#[tokio::test]
async fn bpmux_board_renders_a_compact_index_with_state_filter() {
    let (bpmux, _server) = seeded_bpmux();
    let out = run_bpmux(bpmux.port(), &["board", "--state", "ready"]).await;
    assert!(
        out.status.success(),
        "bpmux board --state ready should exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("board ("),
        "bpmux board renders a compact counts-header index; stdout: {stdout}"
    );
    assert!(
        stdout.contains("#1234") && stdout.contains("ready"),
        "the state-filtered board carries the ready story; stdout: {stdout}"
    );
}
