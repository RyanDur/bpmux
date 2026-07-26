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
        .expect("run bpmux insight-read verb subprocess")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[tokio::test]
async fn bpmux_velocity_reaches_the_gated_rpc_read_off_mcp() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["velocity"]).await;
    assert!(
        out.status.success(),
        "bpmux velocity reaches the same gated /api/rpc/view_velocity read the bridge uses (a 401/404 would fail); stderr: {}",
        stderr(&out)
    );
    let parsed: serde_json::Value =
        serde_json::from_str(stdout(&out).trim()).expect("velocity returns parseable JSON off MCP");
    assert!(
        parsed["board"].get("velocity_window_iterations").is_some()
            && parsed["board"].get("rework").is_some()
            && parsed.get("window_days").is_some(),
        "velocity returns the unified metrics payload off MCP (board + flow halves, not empty / a route-404); stdout: {}",
        stdout(&out)
    );
}

#[tokio::test]
async fn bpmux_commits_reaches_the_gated_rpc_read_for_a_known_story_off_mcp() {
    let stub_dir = tempfile::TempDir::new().expect("stub dir");
    let log_fixture = "abc1234567890abcdef1234567890abcdef1234\u{1f}abc1234\u{1f}Fix #476 substrate work\u{1f}dev\u{1f}2026-05-01T00:00:00+00:00\n";
    std::fs::write(stub_dir.path().join("log.txt"), log_fixture).expect("write log fixture");
    let stub_dir_str = stub_dir.path().to_str().expect("stub dir path").to_string();
    let bpmux = TestBpmux::start_with_env("human", "BPMUX_GIT_STUB_DIR", &stub_dir_str);
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["commits", "476"]).await;
    assert!(
        out.status.success(),
        "bpmux commits reaches /api/rpc/view_story_commits for a real story off MCP; stderr: {}",
        stderr(&out)
    );
    let parsed: serde_json::Value =
        serde_json::from_str(stdout(&out).trim()).expect("commits returns parseable JSON off MCP");
    let arr = parsed
        .as_array()
        .expect("commits returns the git-log array contract off MCP");
    assert!(
        arr.first().and_then(|c| c["subject"].as_str()) == Some("Fix #476 substrate work"),
        "commits surfaces the #476-tagged commit the server's git read found (non-vacuous: the real subject, not an empty array / a 404); stdout: {}",
        stdout(&out)
    );
}

#[tokio::test]
async fn bpmux_diff_reaches_the_gated_rpc_read_for_a_sha_off_mcp() {
    let stub_dir = tempfile::TempDir::new().expect("stub dir");
    let show_fixture = "commit abc1234567890\nAuthor: dev\n\n    Fix #476 substrate work\n\ndiff --git a/src/lib.rs b/src/lib.rs\n@@ -1 +1 @@\n-old\n+new\n";
    std::fs::write(stub_dir.path().join("show.txt"), show_fixture).expect("write show fixture");
    let stub_dir_str = stub_dir.path().to_str().expect("stub dir path").to_string();
    let bpmux = TestBpmux::start_with_env("human", "BPMUX_GIT_STUB_DIR", &stub_dir_str);
    bpmux.simulate_secret();
    let out = run_verb(&bpmux, "dev", &["diff", "abc1234567890"]).await;
    assert!(
        out.status.success(),
        "bpmux diff reaches /api/rpc/view_commit_diff for a sha off MCP; stderr: {}",
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("diff --git a/src/lib.rs"),
        "diff surfaces the patch the server's git read returned (non-vacuous: the real diff body, not empty / a 404); stdout: {}",
        stdout(&out)
    );
}
