#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;

async fn run_capture(bpmux: &TestBpmux, role: &str, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    tokio::process::Command::new(bin)
        .args(args)
        .env("BPMUX_PORT", bpmux.port().to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", bpmux.auth_token_path())
        .output()
        .await
        .expect("run bpmux capture subprocess")
}

fn combined_output(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn capture_error_kind(s: &str) -> Option<String> {
    let needle = "\"kind\":\"";
    let start = s.find(needle)? + needle.len();
    let rest = &s[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn capture_class(s: &str) -> String {
    if let Some(kind) = capture_error_kind(s) {
        format!("error:{kind}")
    } else if s.contains("\"window_id\"") {
        "result".to_string()
    } else {
        "unreached".to_string()
    }
}

#[tokio::test]
async fn bpmux_capture_window_is_byte_same_as_mcp_capture_electron_window() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();

    let bridge = TestBridge::connect(&bpmux, "dev").await;
    let mcp = bridge
        .call_tool("capture_electron_window", serde_json::json!({}))
        .await;
    bridge.close().await;
    let mcp_text = match &mcp {
        Ok(v) => v.to_string(),
        Err(e) => e.clone(),
    };

    let cli = run_capture(&bpmux, "dev", &["capture", "--window"]).await;
    let cli_text = combined_output(&cli);

    let mcp_class = capture_class(&mcp_text);
    assert_ne!(
        mcp_class, "unreached",
        "MCP capture_electron_window must reach the gated /api/electron-capture (auth-propagated bridge) and yield a valid capture contract; got: {mcp_text}"
    );
    assert_eq!(
        capture_class(&cli_text),
        mcp_class,
        "bpmux capture --window must surface the byte-same endpoint response as MCP capture_electron_window (the exact CaptureError kind when no Electron, a CaptureResult when present) — both are peer clients of the one /api/electron-capture endpoint; a 404 (no route) or 401 (no auth) would be 'unreached'. cli={cli_text} mcp={mcp_text}"
    );
}

#[tokio::test]
async fn bpmux_capture_without_window_is_refused_before_reaching_the_endpoint() {
    let bpmux = TestBpmux::start();
    bpmux.simulate_secret();

    let cli = run_capture(&bpmux, "dev", &["capture"]).await;
    let cli_text = combined_output(&cli);

    assert!(
        !cli.status.success(),
        "bare `bpmux capture` (no --window) must be refused — the TUI screen-capture it once mapped to was retired in #1166; cli={cli_text}"
    );
    assert!(
        cli_text.contains("--window"),
        "the refusal must name --window so the user reaches the live Electron-window capture; cli={cli_text}"
    );
    assert_eq!(
        capture_class(&cli_text),
        "unreached",
        "bare capture must NOT reach /api/electron-capture (no CaptureError kind, no CaptureResult) — proving the retired screen-mode was refused outright, not silently re-homed to the live endpoint; cli={cli_text}"
    );
}
