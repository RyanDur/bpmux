use crate::harness::bridge::TestBridge;
use crate::harness::pty::TestBpmux;

const PAUSE_VOCAB_NO_CAUSE: &str = "standing by";
const PAUSE_VOCAB_WITH_CAUSE: &str = "standing by — blocker on red-ci";
const GUARD_REASON_MARKER: &str = "self-imposed-pause";

#[tokio::test]
async fn pause_vocab_without_cause_is_rejected_identically_on_mcp_and_cli() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let dev = TestBridge::connect(&bpmux, "dev").await;
    let mcp_err = dev
        .call_tool(
            "send_message",
            serde_json::json!({ "text": PAUSE_VOCAB_NO_CAUSE }),
        )
        .await
        .expect_err("MCP send_message must hard-reject pause-vocab without a named cause");
    assert!(
        mcp_err.contains(GUARD_REASON_MARKER),
        "the MCP rejection must carry the board-speech-guard reason; got: {mcp_err}"
    );

    let cli = tokio::process::Command::new(bin)
        .args(["send", PAUSE_VOCAB_NO_CAUSE])
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .output()
        .await
        .expect("run bpmux send");
    assert!(
        !cli.status.success(),
        "bpmux send must exit non-zero on the SAME pause-vocab the MCP path rejects — the CLI must not silently bypass the board-speech-guard; status: {:?}, stdout: {}",
        cli.status,
        String::from_utf8_lossy(&cli.stdout)
    );
    let cli_err = String::from_utf8_lossy(&cli.stderr);
    assert!(
        cli_err.contains(GUARD_REASON_MARKER),
        "the CLI rejection must carry the SAME board-speech-guard reason as the MCP path — single-source parity, not a divergent copy; stderr: {cli_err}"
    );
}

#[tokio::test]
async fn pause_vocab_with_named_cause_passes_identically_on_mcp_and_cli() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let bin = env!("CARGO_BIN_EXE_bpmux");

    let dev = TestBridge::connect(&bpmux, "dev").await;
    dev.call_tool(
        "send_message",
        serde_json::json!({ "text": PAUSE_VOCAB_WITH_CAUSE }),
    )
    .await
    .expect("MCP send_message naming a stop-cause must pass the board-speech-guard");

    let cli = tokio::process::Command::new(bin)
        .args(["send", PAUSE_VOCAB_WITH_CAUSE])
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", "dev")
        .output()
        .await
        .expect("run bpmux send");
    assert!(
        cli.status.success(),
        "bpmux send naming a stop-cause must succeed on both paths — the positive control proving the gate blocks the anti-pattern, not every send; stderr: {}",
        String::from_utf8_lossy(&cli.stderr)
    );
}
