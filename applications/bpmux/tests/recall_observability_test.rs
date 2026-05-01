//! #173 — observability for SessionStart recall.
//!
//! AC: when `bpmux recall --role <role>` runs against a live bpmux
//! server, an `activity`-type message appears in the channel log
//! reporting the lesson count. This closes the observability gap
//! Ryan surfaced during #157 acceptance.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// Zero-lesson path — the newly-seeded session has no lessons for
/// 'dev', so recall emits the "no prior lessons" activity line. The
/// test spawns a live bpmux process, runs the recall CLI pointed at
/// its port, then polls `/api/messages` for the activity row.
#[test]
fn recall_emits_activity_message_visible_via_rest() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();

    // (TestBpmux::start now waits for the HTTP listener bind before
    // returning — slice 14 — so we can launch `bpmux recall` directly.)

    // Fresh session base for recall. Decoupled from the TUI's session
    // dir — recall POSTs to the server via port, it doesn't need the
    // same on-disk session.
    let recall_session = tempfile::TempDir::new().expect("temp dir");

    let bin = env!("CARGO_BIN_EXE_bpmux");
    let output = std::process::Command::new(bin)
        .arg("--session-base")
        .arg(recall_session.path())
        .arg("recall")
        .arg("--role")
        .arg("dev")
        .env("BPMUX_PORT", port.to_string())
        .env(
            "BPMUX_LOG",
            std::env::temp_dir().join("bpmux-test-diag.log"),
        )
        .output()
        .expect("run bpmux recall");
    assert!(
        output.status.success(),
        "recall should exit 0. stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Poll for the activity message. The POST is best-effort and
    // local, so it should land within a second; we give it more room
    // for CI variance.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    // chat_only=false so activity-type messages are included in the
    // response (default chat_only=true would hide the recall signal).
    let url = format!("http://127.0.0.1:{port}/api/messages?limit=100&chat_only=false");
    let rt = tokio::runtime::Runtime::new().expect("rt");
    let found = rt.block_on(async {
        loop {
            if std::time::Instant::now() > deadline {
                return None;
            }
            let body = reqwest::get(&url).await.ok()?.text().await.ok()?;
            let msgs: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap_or_default();
            if let Some(hit) = msgs.into_iter().find(|m| {
                m.get("message_type").and_then(|v| v.as_str()) == Some("activity")
                    && m.get("text")
                        .and_then(|v| v.as_str())
                        .is_some_and(|t| t.contains("no prior lessons"))
            }) {
                return Some(hit);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let hit = found.expect("recall activity should appear in channel log");
    let from = hit["from"].as_str().unwrap_or("");
    assert_eq!(from, "dev:claude-code", "activity sender names the role");
    let text = hit["text"].as_str().unwrap_or("");
    assert!(
        text.contains("dev"),
        "activity text should name the role. got: {text}"
    );
}
